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
	clientList: Mutex<Vec<Vec<u8>>>,
	clientProfiles: Mutex<HashMap<Vec<u8>, Profile>>,
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
			match threadReciever.recv() {
				Ok(cnt) => {
					/* worker thread send stateNum 
					when finish processing one client
					*/
					println!("Server mpsc recieved {:?}", cnt);
					let mut stateGuard = self.STATE.write().unwrap();
					if cnt == *stateGuard {
						recvCnt += 1;
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
								res = publish(&publisher,
											"Finished",
											"HS")
							},
							1 => {
								println!("sent keys");

								res = publish_vecs(&publisher,
											format_clientData(&(*profilesGuard), &(*listGuard), "publicKey").unwrap(),
											"KE");
								let sharingParams = self.param.calculate_sharing(self.MAX, self.BLOCK);
								let mut spBytes = Vec::new();
								for sp in sharingParams {
									spBytes.push(sp.to_le_bytes().to_vec());
								}
								res = publish_vecs(&publisher,
											spBytes,
											"IS")
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
				},
				Err(_) => {
					continue;
				},
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
			println!("	client message: {:?}", msg);

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
		{
			if *self.STATE.read().unwrap() != 1 {						// Correct State
				send(&worker.dealer, 
					"Error: Wrong state.", 
					&clientID);
				return Err(WorkerError::WrongState(1))
			}
			match self.clientProfiles.lock() {							// Mutex Obtained
				Ok(guard) => profilesGuard = guard,
				Err(guard) => return Err(WorkerError::MutexLockFail(0)),
			};
			match (*profilesGuard).get_mut(&clientID) {					// Existing Client
			 	Some(p) => profile = p,
			 	None => {
			 		send(&worker.dealer, 
			 			"Error: Your profile is not found.", 
			 			&clientID);		
			 		return Err(WorkerError::ClientNotFound(0))
			 	},
			};
		}
		
		/*-------------------- Checks Finished --------------------*/
		
		match msg {
			RecvType::matrix(m) => {
				if (m.len() != 2) {
					send(&worker.dealer, 
						"Please send your public key with a signature.Format: [publicKey, Enc(publicKey)]", 
						&clientID);
					return Err(WorkerError::UnexpectedFormat(0))
				}
				let publicKey = &m[0];
				let singedPublicKey = &m[1];
				let verifyResult = profile.veriKey.verify(
					publicKey, 
					&Signature::from_bytes(singedPublicKey).unwrap());
				match verifyResult {
					Ok(_) => {
						profile.publicKey = publicKey.to_vec();
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
			},
			_ => {
				send(&worker.dealer, 
					"Please send your verification key with a signature.Format: [publicKey, Enc(publicKey, veriKey)]", 
					&clientID);
				return Err(WorkerError::UnexpectedFormat(0))
			},
		}
		println!("key_exchanged with {:?}", std::str::from_utf8(&clientID).unwrap());
		return Ok(1)
	}

	fn inpus_sharing(&self, 
		worker: &Worker, clientID: Vec<u8>, msg: RecvType) -> Result<usize, WorkerError> {
		
		let mut profile;
		let mut profilesGuard;
		{
			if *self.STATE.read().unwrap() != 1 {						// Correct State
				send(&worker.dealer, 
					"Error: Wrong state.", 
					&clientID);
				return Err(WorkerError::WrongState(1))
			}
			match self.clientProfiles.lock() {							// Mutex Obtained
				Ok(guard) => profilesGuard = guard,
				Err(guard) => return Err(WorkerError::MutexLockFail(0)),
			};
			match (*profilesGuard).get_mut(&clientID) {					// Existing Client
			 	Some(p) => profile = p,
			 	None => {
			 		send(&worker.dealer, 
			 			"Error: Your profile is not found.", 
			 			&clientID);		
			 		return Err(WorkerError::ClientNotFound(0))
			 	},
			};
		}

		/*-------------------- Checks Finished --------------------*/

		match msg {
			RecvType::matrix(m) => {
				return Ok(2)
			},
			_ => {
				send(&worker.dealer, "Please send your shares.", &clientID);
				return Err(WorkerError::UnexpectedFormat(0))
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
		            "publicKey" => data.unwrap().publicKey.clone(),
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