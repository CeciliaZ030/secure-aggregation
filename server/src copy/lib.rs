use std::collections::HashMap;
use std::str;
use std::thread;
use std::thread::JoinHandle;
use std::sync::*;
use std::convert::TryInto;

use rand_core::OsRng;

use zmq::Message;
use zmq::SNDMORE;

use signature::Signature as _;
use p256::NistP256;
use p256::{
    ecdsa::{SigningKey, Signature, signature::Signer},
    ecdsa::{VerifyKey, signature::Verifier},
};

use packed_secret_sharing::packed::*;

mod sockets;
pub mod worker;
pub mod param;
use sockets::*;
use worker::*;
use param::*;

pub struct Server {
	STATE: RwLock<usize>,
	MAX: usize,
	V: usize,
	param: Param,
	clientList: Mutex<Vec<Vec<u8>>>,					// array of ID
	clientProfiles: Mutex<HashMap<Vec<u8>, Profile>>,	// key = ID, value = Profile
	keyList: RwLock<HashMap<Vec<u8>, Vec<u8>>>,			// key = pubKey, value = ID for input sharing retrival
	shares: Mutex<Vec<Vec<u64>>>,
	result: Vec<u64>,
}


#[derive(Debug)]
pub struct Profile {
	veriKey: VerifyKey,
	publicKey:  Vec<u8>,
	hasShared: bool,
}

impl Server {

	pub fn new(maxClients: usize, vectorSize: usize, mut param: Param) -> Server {
		param.calculate(maxClients, blockLength);
		Server {
			STATE: RwLock::new(0usize),
			MAX: maxClients,
			V: blockLength,
			param: param,
			clientList: Mutex::new(Vec::new()),
			clientProfiles: Mutex::new(HashMap::<Vec<u8>, Profile>::new()),
			keyList: RwLock::new(HashMap::<Vec<u8>, Vec<u8>>::new()),
			shares: Mutex::new(Vec::new()),
			result: Vec::new(),
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
			if (*stateGuard == 2 && recvCnt >= self.MAX*self.MAX) || (*stateGuard != 2 && recvCnt >= self.MAX) {

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

				let mut res;
				match *stateGuard {
					0 => {
						res = publish_vecs(&publisher,
								format_clientData(&(*profilesGuard), &(*listGuard), "veriKey").unwrap(),
								"HS")
					},
					1 => {
						res = publish_vecs(&publisher,
								format_clientData(&(*profilesGuard), &(*listGuard), "publicKey").unwrap(),
								"KE");
						let sharingParams = self.param.send();
						println!("sharingParams {:?}", sharingParams);
						let mut spBytes = Vec::new();
						for sp in sharingParams {
							spBytes.extend(sp.to_le_bytes().to_vec());
						}
						res = publish(&publisher, spBytes, "IS")
					},
					2 => res = publish(&publisher, "Please send your aggregated shares.", "AG"),
					3 => res = self.reconstruction(),
					_ => res = Ok(0),
				};
				match res {
					Ok(_) => {
						*stateGuard += 1;
						println!("Server: STATE change from {:?} to {:?}", *stateGuard-1, *stateGuard);
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
				2 => self.input_sharing(&worker, clientID, msg),
				3 => self.shares_collection(&worker, clientID, msg),
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

		let verifyResult = profile.veriKey.verify(
			&publicKey, 
			&Signature::from_bytes(&singedPublicKey).unwrap()
		);

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

	fn input_sharing(&self, 
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
					m[1].len(), str::from_utf8(&clientID).unwrap(), str::from_utf8(&sendTo).unwrap());
				
				let msg = vec![profile.publicKey.clone(), m[1].clone()];
				match send_vecs(&worker.dealer, msg, &sendTo) {
					Ok(_) => {
						send(&worker.dealer,
							format!("sharing from {:?} to {:?} successful", 
								str::from_utf8(&clientID).unwrap(),
								str::from_utf8(&sendTo).unwrap()).as_str(),
							&clientID);
						worker.threadSender.send(2);
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

	fn shares_collection(&self, 
		worker: &Worker, clientID: Vec<u8>, msg: RecvType) -> Result<usize, WorkerError> {
		println!("shares_collection");
		let mut profile;
		let mut profilesGuard;
		let mut sharesGuard;
		{
			if *self.STATE.read().unwrap() != 3 {						// Correct State
				send(&worker.dealer, 
					"Error: Wrong state.", 
					&clientID);
				return Err(WorkerError::WrongState(3))
			}
			match self.clientProfiles.lock() {							// Mutex Obtained
				Ok(guard) => profilesGuard = guard,
				Err(guard) => return Err(WorkerError::MutexLockFail(3)),
			};
			match (*profilesGuard).get_mut(&clientID) {					// Existing Client
			 	Some(p) => profile = p,
			 	None => {
			 		send(&worker.dealer, 
			 			"Error: Your profile is not found.", 
			 			&clientID);		
			 		return Err(WorkerError::ClientNotFound(3))
			 	},
			};
			match self.shares.lock() {
				Ok(guard) => sharesGuard = guard,
				Err(guard) => return Err(WorkerError::MutexLockFail(3)),
			}
		}
		match msg {
			RecvType::matrix(m) => {
				if (m.len() != 2) {
					send(&worker.dealer, 
						"Please send your shares with a signature. Format: [shares, Enc(shares)]", 
						&clientID);
					return Err(WorkerError::UnexpectedFormat(3))
				}
				let verifyResult = profile.veriKey.verify(
					&m[0], 
					&Signature::from_bytes(&m[1]).unwrap()
				);
				match verifyResult {
					Ok(_) => {
						sharesGuard.push(read_le_u64(&m[0]));
				 		send(&worker.dealer, 
				 			"Your aggregated shares has been save.", 
				 			&clientID);
				 		println!("Yeah!");
				 		worker.threadSender.send(3);
				 		return Ok(3)
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
					"Please send your shares key with a signature. Format: [shares, Enc(shares)]", 
					&clientID);
				return Err(WorkerError::UnexpectedFormat(0))
			},
		}

	}

	fn reconstruction(&self) -> Result<usize, usize> {
		println!("in reconstruction");
		let mut sharesGuard;
		{
			match self.shares.lock() {									// Mutex Obtained
				Ok(guard) => sharesGuard = guard,
				Err(_) => return Err(3),
			};
		}		
		/*-------------------- Checks Finished --------------------*/

		// number of blocks to reconcstruct
		let B = sharesGuard[0].len();
		let N = sharesGuard.len();
		let param = self.param;
		for i in 0..B {
			// When V = B * L + remains
			if i == B-1 && (param.useDegree2 as usize) * B != self.V {
				let mut pss = PackedSecretSharing::new(
					param.P, param.R2, param.R3, 
					param.useDegree2, param.useDegree3, self.V-(param.useDegree2 as usize)*B, N
					);
			}
			else {
				let mut pss = PackedSecretSharing::new(
					self.param.P, self.param.R2, self.param.R3, 
					param.useDegree2, param.useDegree3, param.useDegree2, N
				);
			}
			let mut shares = Vec::new();
			for j in 0..N {
				shares.push(sharesGuard[j][i] as u128);
			}
			self.result.extend(pss.reconcstruct(&shares));
		}
		return Ok(3);

	}
}
