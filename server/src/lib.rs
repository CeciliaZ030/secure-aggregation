use std::collections::HashMap;
use std::str;
use std::thread;
use std::thread::JoinHandle;
use std::sync::*;

use rand_core::OsRng;

use zmq::Message;
use zmq::SNDMORE;

use signature::Signature as _;
use p256::NistP256;
use p256::{
    ecdsa::{SigningKey, Signature, signature::Signer},
    ecdsa::{VerifyKey, signature::Verifier},
};

mod sockets;
pub mod param;
use sockets::*;
use param::*;

pub struct Server{
	STATE: RwLock<usize>,
	MAX: usize,
	BLOCK: usize,
	param: Param,
	clientList: Mutex<Vec<Vec<u8>>>,					// array of ID
	clientProfiles: Mutex<HashMap<Vec<u8>, Profile>>,	// key = ID, value = Profile
	keyList: RwLock<HashMap<Vec<u8>, Vec<u8>>> 			// key = pubKey, value = ID for input sharing retrival
}


#[derive(Debug)]
pub struct Profile {
	veriKey: VerifyKey,
	publicKey:  Vec<u8>,
	hasShared: bool,
}

#[derive(Debug)]
pub enum WorkerError {
	MutexLockFail(usize),
	ClientNotFound(usize),
	ClientExisted(usize),
	MaxClientExceed(usize),
	DecryptionFail(usize),
	UnexpectedFormat(usize),
	UnknownState(usize),
	WrongState(usize),
	SharingFail(usize)
}

#[derive(Debug)]
pub enum ServerError {
	MissingClient(usize),
	MutexLockFail(usize),
	ThreadJoinFail(usize),
	FailPublish(usize),
	UnexpectedField(usize),
}

impl Server {

	pub fn new(maxClients: usize, blockLength: usize, param: Param) -> Server {
		Server {
			STATE: RwLock::new(0usize),
			MAX: maxClients,
			BLOCK: blockLength,
			param: param,
			clientList: Mutex::new(Vec::new()),
			clientProfiles: Mutex::new(HashMap::<Vec<u8>, Profile>::new()),
			keyList: RwLock::new(HashMap::<Vec<u8>, Vec<u8>>::new())
		}
	}

	pub fn server_task(&self, 
		context: zmq::Context, port1: usize) -> Result<usize, ServerError>  {

		let frontend = context.socket(zmq::ROUTER).unwrap();
    	let backend = context.socket(zmq::DEALER).unwrap();

		assert!(frontend
			.bind(&format!("tcp://*:{:?}", port1))
			.is_ok());
		assert!(backend
			.bind("inproc://backend")
			.is_ok());

		zmq::proxy(&frontend, &backend);
		return Ok(0)
	}

	pub fn state_task(&self, 
		context: zmq::Context, port2: usize, threadReciever: mpsc::Receiver<usize>) -> Result<usize, ServerError> {

    	println!("{}", &format!("tcp://*:{:?}", port2));
		let publisher = context.socket(zmq::PUB).unwrap();
        publisher.set_sndhwm(1_100_000).expect("failed setting hwm");
		assert!(publisher
			.bind(&format!("tcp://*:{:?}", port2))
			.is_ok());

		let mut recvCnt = 0;
		loop {
			let mut stateGuard;
			match threadReciever.recv() {
				Ok(cnt) => {
					/* worker thread send stateNum 
					when finish processing one client
					*/
					println!("Server mpsc recieved {:?}", cnt);
					stateGuard = self.STATE.write().unwrap();
					if cnt == *stateGuard {
						recvCnt += 1;
					}
				},
				Err(_) => {
					continue;
				},
			}
			/* when finished client num exceed MAX
			initiate state change
			*/
			if recvCnt >= self.MAX {

				let mut profilesGuard;
				let mut listGuard;
				match self.clientProfiles.lock() {							// Mutex Obtained
					Ok(guard) => profilesGuard = guard,
					Err(_) => return Err(ServerError::MutexLockFail(0)),
				}; 
				match self.clientList.lock() {
					Ok(guard) => listGuard = guard,
					Err(_) => return Err(ServerError::MutexLockFail(0)),
				};

				println!("here ++++++++++++\n\n");
				let mut res;
				match *stateGuard {
					0 => {
						println!("sent");
						res = publish_vecs(&publisher,
								format_clientData(&(*profilesGuard), &(*listGuard), "veriKey").unwrap(),
								"HS")
					},
					1 => {
						println!("sent keys");

						res = publish_vecs(&publisher,
								format_clientData(&(*profilesGuard), &(*listGuard), "publicKey").unwrap(),
								"KE");
						let sharingParams = self.param.calculate_sharing(self.MAX, self.BLOCK);
						println!("sharingParams {:?}", sharingParams);
						let mut spBytes = Vec::new();
						for sp in sharingParams {
							spBytes.push(sp.to_le_bytes().to_vec());
						}
						res = publish_vecs(&publisher, spBytes, "IS")
					},
					_ => res = Ok(0),
				};
				match res {
					Ok(_) => {
						*stateGuard += 1;
						println!("Server: STATE change\n {:?}", *profilesGuard);
						recvCnt = 0;
					},
					Err(_) => return Err(ServerError::FailPublish(0)),
				};
			}
		}
		return Ok(0)
	}

	pub fn worker_task(&self, worker: Worker)-> Result<usize, ServerError> {
		loop {
			let clientID = take_id(&worker.dealer);

			println!("{} Taken {:?}", 
				worker.ID, 
				String::from_utf8(clientID.clone()).unwrap());

			let msg = recv(&worker.dealer);

			match *(self.STATE.read().unwrap()) {
				0 => self.handshake(&worker, clientID, msg),
				1 => self.key_exchange(&worker, clientID, msg),
				2 => self.inpus_sharing(&worker, clientID, msg),
				_ => Err(WorkerError::UnknownState(0))
			};
		}
		return Ok(0)
	}

	fn handshake(&self, 
		worker: &Worker, clientID: Vec<u8>, msg: RecvType) -> Result<usize, WorkerError> {

		let mut profilesGuard;
		let mut listGuard;
		{
			if *self.STATE.read().unwrap() != 0 {						// Correct State
				send(&worker.dealer, 
					"Error: Wrong state.", 
					&clientID);
				return Err(WorkerError::WrongState(0))
			}
			match self.clientProfiles.lock() {							// Mutex Obtained
				Ok(guard) => profilesGuard = guard,
				Err(guard) => return Err(WorkerError::MutexLockFail(0)),
			};
			match self.clientList.lock() {
				Ok(guard) => listGuard = guard,
				Err(guard) => return Err(WorkerError::MutexLockFail(0)),
			};
			if (*profilesGuard).contains_key(&clientID) || (*listGuard).contains(&clientID) {				
																		// New Client
				send(&worker.dealer, 
					"Error: You already exists.", 
					&clientID);
				println!("Error: You already exists.");
				return Err(WorkerError::ClientExisted(0))
			}
			if (*profilesGuard).len() == self.MAX {						// Under Limit
	            send(&worker.dealer, 
	            	"Error: Reached maximun client number.", 
	            	&clientID);
				return Err(WorkerError::MaxClientExceed(0))
			}
		}

		/*-------------------- Checks Finished --------------------*/

		let signKey = SigningKey::random(&mut OsRng);
		let veriKey = VerifyKey::from(&signKey);
		send(&worker.dealer,
		 	SigningKey::to_bytes(&signKey).to_vec(), 
		 	&clientID);

		(*profilesGuard).insert(
			clientID.clone(),
			Profile{
				veriKey: veriKey,
				publicKey: Vec::new(),
				hasShared: false,
			}
		);
		(*listGuard).push(clientID.clone());
		worker.threadSender.send(0);

		println!("handshaked with {:?}", std::str::from_utf8(&clientID).unwrap());
		return Ok(1)
	}

	fn key_exchange(&self, 
		worker: &Worker, clientID: Vec<u8>, msg: RecvType) -> Result<usize, WorkerError> {
		
		let mut profile;
		let mut profilesGuard;
		let mut keyListGuard;
		{
			if *self.STATE.read().unwrap() != 1 {						// Correct State
				send(&worker.dealer, 
					"Error: Wrong state.", 
					&clientID);
				return Err(WorkerError::WrongState(1))
			}
			match self.clientProfiles.lock() {							// Mutex Obtained
				Ok(guard) => profilesGuard = guard,
				Err(_) => return Err(WorkerError::MutexLockFail(1)),
			};
			match (*profilesGuard).get_mut(&clientID) {					// Existing Client
			 	Some(p) => profile = p,
			 	None => {
			 		send(&worker.dealer, 
			 			"Error: Your profile is not found.", 
			 			&clientID);		
			 		return Err(WorkerError::ClientNotFound(1))
			 	},
			};
			match self.keyList.write() {
				Ok(guard) => keyListGuard = guard,
				Err(_) => return Err(WorkerError::MutexLockFail(1)),
			}
		}
		
		/*-------------------- Checks Finished --------------------*/
		let publicKey;
		let singedPublicKey;		
		match msg {
			RecvType::matrix(m) => {
				if (m.len() != 2) {
					send(&worker.dealer, 
						"Please send your public key with a signature.Format: [publicKey, Enc(publicKey)]", 
						&clientID);
					return Err(WorkerError::UnexpectedFormat(0))
				}
				publicKey = m[0].clone();
				singedPublicKey = m[1].clone();
			},
			_ => {
				send(&worker.dealer, 
					"Please send your verification key with a signature.Format: [publicKey, Enc(publicKey, veriKey)]", 
					&clientID);
				return Err(WorkerError::UnexpectedFormat(0))
			},
		}
		let verifyResult = profile.veriKey
								  .verify(&publicKey, &Signature::from_bytes(&singedPublicKey).unwrap());
		match verifyResult {
			Ok(_) => {
				profile.publicKey = publicKey.to_vec();
				keyListGuard.insert(publicKey.to_vec(), clientID.clone());
		 		send(&worker.dealer, 
		 			"Your publicKey has been save.", 
		 			&clientID);
		 		worker.threadSender.send(1);
			},
			Err(_) => {
		 		send(&worker.dealer, 
		 			"Error: Decryption Fail.", 
		 			&clientID);
				return Err(WorkerError::DecryptionFail(0))
			},
		}
		println!("key_exchanged with {:?}", std::str::from_utf8(&clientID).unwrap());
		return Ok(1)
	}

	// TODO: 把profile做成 Mutex<HashMap<Vec<u8>, Mutex<Profile>>>
	/*
		收新client的时候get整个hash map的mutex
		后面单独取一个profile的mutex就行
		--> less thread blocking
	*/

	fn inpus_sharing(&self, 
		worker: &Worker, clientID: Vec<u8>, msg: RecvType) -> Result<usize, WorkerError> {
		
		let mut profile;
		let mut profilesGuard;
		let mut keyListGuard;
		{
			if *self.STATE.read().unwrap() != 2 {						// Correct State
				send(&worker.dealer, 
					"Error: Wrong state.", 
					&clientID);
				return Err(WorkerError::WrongState(2))
			}
			match self.clientProfiles.lock() {							// Mutex Obtained
				Ok(guard) => profilesGuard = guard,
				Err(guard) => return Err(WorkerError::MutexLockFail(2)),
			};
			match (*profilesGuard).get_mut(&clientID) {					// Existing Client
			 	Some(p) => profile = p,
			 	None => {
			 		send(&worker.dealer, 
			 			"Error: Your profile is not found.", 
			 			&clientID);		
			 		return Err(WorkerError::ClientNotFound(2))
			 	},
			};
			match self.keyList.read() {
				Ok(guard) => keyListGuard = guard,
				Err(_) => return Err(WorkerError::MutexLockFail(2)),
			}
		}

		/*-------------------- Checks Finished --------------------*/

		match msg {
			RecvType::matrix(m) => {
				let sendTo = match keyListGuard.get(&m[0]) {
					Some(id) => id,
					None => return Err(WorkerError::ClientNotFound(2)),
				};
				println!("input sharing matix (len: {:?}) from {:?} to {:?}", 
					m.len(), str::from_utf8(&clientID).unwrap(), str::from_utf8(&sendTo).unwrap());
				
				let msg = vec![profile.publicKey.clone(), m[1].clone()];
				match send_vecs(&worker.dealer, msg, &sendTo) {
					Ok(_) => {
						send(&worker.dealer,
							format!("sharing from {:?} to {:?} successful", 
								str::from_utf8(&clientID).unwrap(),
								str::from_utf8(&sendTo).unwrap()).as_str(),
							&clientID);
						return Ok(2)
					},
					Err(_) => return Err(WorkerError::SharingFail(2)),
				};
			}, 
			_ => {
				send(&worker.dealer, "Please send your shares.", &clientID);
				return Err(WorkerError::UnexpectedFormat(2))
			},
		}
	}

}

pub struct Worker {
	ID: String,
	dealer : zmq::Socket,
	threadSender: mpsc::Sender<usize>,
}

impl Worker {
	pub fn new(ID: &str, 
		context: zmq::Context, threadSender: mpsc::Sender<usize>) -> Worker {
		let dealer = context.socket(zmq::DEALER).unwrap();
		dealer.set_identity(ID.as_bytes());
		assert!(dealer.connect("inproc://backend").is_ok());
		Worker {
			ID: ID.to_string(),
			dealer: dealer,
			threadSender: threadSender,
		}
	}
}

pub fn format_clientData(datas: &HashMap<Vec<u8>, Profile>, 
    order: &Vec<Vec<u8>>, field: &str) -> Result<Vec<Vec<u8>>, ServerError> {
    //println!("order {:?}", order);
    let mut vecs = Vec::new();
    for key in order {
        let data = datas.get(key);
        match data {
        	Some(d) => {
		        let res = match field {
		        	"veriKey" => VerifyKey::to_encoded_point(&d.veriKey, true)
		        					.to_bytes()
		        					.to_vec(),
		            "publicKey" => d.publicKey.clone(),
		            _ => return Err(ServerError::UnexpectedField(0)),
		        };
		        vecs.push(res);
        	},
        	None => {
        		return Err(ServerError::MissingClient(0))
        	},
        }
    } 
    //print!("format_clientData {:?}", vecs);
    return Ok(vecs)
}
