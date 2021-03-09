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
use p256::{
	NistP256,
    ecdsa::{
    	SigningKey, Signature, signature::Signer,
    	VerifyKey, signature::Verifier
    }
};

use packed_secret_sharing::packed::*;

mod sockets;
pub mod worker;
pub mod param;
use sockets::*;
use param::*;
use worker::*;
use worker::ServerError;
use worker::WorkerError;


#[derive(Debug)]
pub struct Profile {
	veriKey: VerifyKey,
	publicKey:  Vec<u8>,
	hasShared: bool,
}

pub struct Server {
	STATE: RwLock<usize>,
	MAX: usize,
	V: usize,
	param: Param,
	clientList: RwLock<Vec<Vec<u8>>>,					// array of ID
	clientProfiles: RwLock<HashMap<Vec<u8>, Profile>>,	// key = ID, value = Profile
	keyList: RwLock<HashMap<Vec<u8>, Vec<u8>>>,			// key = pubKey, value = ID for input sharing retrival
	shares: Mutex<Vec<Vec<u64>>>,
}


impl Server {

	pub fn new(maxClients: usize, vectorSize: usize, mut param: Param) -> Server {
		param.calculate(maxClients, vectorSize);
		Server {
			STATE: RwLock::new(0usize),
			MAX: maxClients,
			V: vectorSize,
			param: param,
			clientList: RwLock::new(Vec::new()),
			clientProfiles: RwLock::new(HashMap::<Vec<u8>, Profile>::new()),
			keyList: RwLock::new(HashMap::<Vec<u8>, Vec<u8>>::new()),
			shares: Mutex::new(Vec::new()),
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
		let mut finalResult;
		loop {
			let mut stateGuard;
			let r = threadReciever.recv();
			println!("loop recv {:?}", r);
			match r {
				Ok(alarm) => {
					/* worker thread send stateNum 
					when finish processing one client
					*/
					println!("Server mpsc recieved alarm {:?}, cnt {}", alarm, recvCnt);
					stateGuard = self.STATE.write().unwrap();
					if alarm == *stateGuard {
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
			if (*stateGuard != 2 && recvCnt >= self.MAX) || (*stateGuard == 2 && recvCnt >= self.MAX*self.MAX) {
				
				let profilesGuard = match self.clientProfiles.write() {
					Ok(mut guard) => guard,
					Err(_) => return Err(ServerError::MutexLockFail(0)),
				}; 
				let listGuard = match self.clientList.write() {
					Ok(mut guard) => guard,
					Err(_) => return Err(ServerError::MutexLockFail(0)),
				};
				let res = match *stateGuard {
					0 => publish_vecs(
							&publisher, 
							format_clientData(&(*profilesGuard), &(*listGuard), "veriKey").unwrap(), 
							"HS"),
					1 => {
						publish_vecs(
							&publisher,
							format_clientData(&(*profilesGuard), &(*listGuard), "publicKey").unwrap(), 
							"KE");
						let sharingParams = self.param.send();
						println!("sharingParams {:?}", sharingParams);
						let mut spBytes = Vec::new();
						for sp in sharingParams {
							spBytes.extend(sp.to_le_bytes().to_vec());
						}
						publish(&publisher, spBytes, "IS")
					},
					2 => publish(&publisher, "Please send your aggregated shares.", "AG"),
					3 => {
						finalResult = self.reconstruction();
						break;
						Ok(0)
					},
					_ => Err(0),
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
	/*
		Check client not existed & under limit
		Record ID and release lock
	*/
		match self.clientList.write() {
			Ok(mut guard) => {
				if guard.contains(&clientID) {
					send(&worker.dealer, "Error: You already exists.", &clientID);
					return Err(WorkerError::MaxClientExceed(0));
				}
				if guard.len() == self.MAX {
		            send(&worker.dealer, "Error: Reached maximun client number.", &clientID);
					return Err(WorkerError::MaxClientExceed(0));	
				}
				// if !self.check_state(0) { 
				// 	send(&worker.dealer,"Error: Wrong state.", &clientID);
				// 	return Err(WorkerError::WrongState(0)); 
				// }
				guard.push(clientID.clone());
			},
			Err(_) => return Err(WorkerError::MutexLockFail(0)),
		};

	/*
		Generate ECDSA keys and send
		Create new profile
		Write to mutex (don't need check_state)
	*/

		let signKey = SigningKey::random(&mut OsRng);
		let veriKey = VerifyKey::from(&signKey);
		send(&worker.dealer,
		 	SigningKey::to_bytes(&signKey).to_vec(), 
		 	&clientID
		 );

		let newProfiel = Profile { 
			veriKey: veriKey, 
			publicKey: Vec::new(), 
			hasShared: false,
		};
		match self.clientProfiles.write() {
			Ok(mut guard) => guard.insert( clientID.clone(), newProfiel),
			Err(guard) => return Err(WorkerError::MutexLockFail(0)),
		};
		worker.threadSender.send(0);
		println!("handshaked with {:?}", std::str::from_utf8(&clientID).unwrap());
		return Ok(1);
	}

	fn key_exchange(&self, 
		worker: &Worker, clientID: Vec<u8>, msg: RecvType) -> Result<usize, WorkerError> {
	/*
		Make sure client exist
		Parse msg
	*/
		// if !self.check_exist(&clientID) {
		// 	send(&worker.dealer,"Error: Your profile not found", &clientID);
		// 	return Err(WorkerError::ClientNotFound(1))
		// }
		let publicKey;
		let singedPublicKey;
		match msg {
			RecvType::matrix(m) => {
				if (m.len() != 2) {
					send(&worker.dealer, 
						"Please send with format: [publicKey, Enc(publicKey)]", &clientID);
					return Err(WorkerError::UnexpectedFormat(1))
				}
				publicKey = m[0].clone();
				singedPublicKey = Signature::from_bytes(&m[1]).unwrap();
			},
			_ => {
				send(&worker.dealer, 
					"Please send with ormat: [publicKey, Enc(publicKey, veriKey)]", &clientID);
				return Err(WorkerError::UnexpectedFormat(0))
			},
		}
	/*
		Clone veriKey
		Verify signature
		Sotre DH pk in profiles & keyList
	*/
		let veriKey = match self.clientProfiles.read() {
			Ok(mut guard) => guard.get(&clientID).unwrap().veriKey.clone(),
			Err(_) => return Err(WorkerError::MutexLockFail(1)),
		};
		match veriKey.verify(&publicKey, &singedPublicKey) {
			Ok(_) => {
				// if !self.check_state(1) {
			 // 		send(&worker.dealer,"Error: Wrong state.", &clientID);
			 // 		return Err(WorkerError::WrongState(1));
			 // 	}
			 	match self.clientProfiles.write() {
					Ok(mut guard) => guard.get_mut(&clientID).unwrap().publicKey = publicKey.to_vec(),
					Err(_) => return Err(WorkerError::MutexLockFail(1)),
				};
				match self.keyList.write() {
					Ok(mut guard) => guard.insert(publicKey.to_vec(), clientID.clone()),
					Err(_) => return Err(WorkerError::MutexLockFail(1)),
				};
		 		send(&worker.dealer, "Your publicKey has been save.", &clientID);
			},
			Err(_) => {
		 		send(&worker.dealer, "Error: Decryption Fail.", &clientID);
				return Err(WorkerError::DecryptionFail(1))
			},
		}
 		worker.threadSender.send(1);
 		println!("key_exchanged with {:?}", std::str::from_utf8(&clientID).unwrap());
		return Ok(1)
	}


	fn input_sharing(&self, 
		worker: &Worker, clientID: Vec<u8>, msg: RecvType) -> Result<usize, WorkerError> {
	/*
		Check client exiists
		Get the share and delivery target
	*/
		if !self.check_exist(&clientID) {
			send(&worker.dealer,"Error: Your profile not found", &clientID);
			return Err(WorkerError::ClientNotFound(1))
		}
		let mut sendTo;
		let msg = match msg {
			RecvType::matrix(m) => {
				sendTo = match self.keyList.read().unwrap().get(&m[0]) {
					Some(id) => id.clone(),
					None => return Err(WorkerError::ClientNotFound(2)),
				};
				let publicKey = match self.clientProfiles.read() {
					Ok(mut guard) => guard.get(&clientID).unwrap().publicKey.clone(),
					Err(_) => return Err(WorkerError::MutexLockFail(1)),
				};
				vec![publicKey, m[1].clone()]
			}, 
			_ => {
				send(&worker.dealer, "Please send your shares as matrix.", &clientID);
				return Err(WorkerError::UnexpectedFormat(2))
			},
		};
	/*
		Send [pk, share] to target
	*/
		println!("input sharing matix (len: {:?}) from {:?} to {:?}", 
					msg[1].len(), str::from_utf8(&clientID).unwrap(), str::from_utf8(&sendTo).unwrap());
		match send_vecs(&worker.dealer, msg, &sendTo) {
			Ok(_) => {
				send(&worker.dealer,
					format!("sharing from {:?} to {:?} successful", 
						str::from_utf8(&clientID).unwrap(),
						str::from_utf8(&sendTo).unwrap()
					).as_str(),
					&clientID);
			},
			Err(_) => return Err(WorkerError::SharingFail(2)),
		};
		worker.threadSender.send(2);
		return Ok(2)
	}


	fn shares_collection(&self, 
		worker: &Worker, clientID: Vec<u8>, msg: RecvType) -> Result<usize, WorkerError> {
	/*
		Check client exist
		Get shares & signature
	*/
		if !self.check_exist(&clientID) {
			send(&worker.dealer,"Error: Your profile not found", &clientID);
			return Err(WorkerError::ClientNotFound(3))
		}
		let msg = match msg {
			RecvType::matrix(m) => {
				if (m.len() != 2) {
					send(&worker.dealer, 
						"Please send your shares with a signature. Format: [shares, Enc(shares)]", 
						&clientID);
					return Err(WorkerError::UnexpectedFormat(3))
				}
				m
			},
			_ => {
				send(&worker.dealer, 
					"Please send your shares key with a signature. Format: [shares, Enc(shares)]", 
					&clientID);
				return Err(WorkerError::UnexpectedFormat(0))
			},
		};
	/*
		Verify & Safe
	*/
		let verifyResult = match self.clientProfiles.read() {
			Ok(mut guard) => {
			 	let veriKey = guard.get(&clientID).unwrap().veriKey.clone();
				veriKey.verify(
					&msg[0], 										//shares
					&Signature::from_bytes(&msg[1]).unwrap()		//signature of shares
				)
			},
			Err(_) => return Err(WorkerError::MutexLockFail(1)),
		};
		match verifyResult {
			Ok(_) => {
				self.shares.lock().unwrap().push(read_le_u64(&msg[0]));
		 		send(&worker.dealer, 
		 			"Your aggregated shares has been save.", 
		 			&clientID);
		 		worker.threadSender.send(3);
		 		return Ok(3)
			},
			Err(_) => {
		 		send(&worker.dealer, "Error: Decryption Fail.", &clientID);
				return Err(WorkerError::DecryptionFail(0))
			},
		}		

	}


	fn reconstruction(&self) -> Result<Vec<u128>, usize> {
		let sharesGuard = match self.shares.lock() {									// Mutex Obtained
			Ok(mut guard) => guard,
			Err(_) => return Err(3),
		};		
		let B = sharesGuard[0].len();
		let N = sharesGuard.len();
		let param = &self.param;
		let mut result = Vec::new();
		for i in 0..B {
			let mut pss;
			// When V = B * L + remains
			if i == B-1 && (param.useDegree2 as usize) * B != self.V {
				pss = PackedSecretSharing::new(
					param.P, param.R2, param.R3, 
					param.useDegree2, param.useDegree3, self.V-(param.useDegree2 as usize)*B, N
					);
			}
			else {
				pss = PackedSecretSharing::new(
					self.param.P, self.param.R2, self.param.R3, 
					param.useDegree2, param.useDegree3, param.useDegree2, N
				);
			}
			let mut shares = Vec::new();
			for j in 0..N {
				shares.push(sharesGuard[j][i] as u128);
			}
			result.extend(pss.reconstruct(&shares));
		}
		println!("Reconstruction DONE");
		return Ok(result);
	}

	fn check_state(&self, state: usize) -> bool {
		if *self.STATE.read().unwrap() == state {return true;}
		return false;
	}
	fn check_exist(&self, clientID: &Vec<u8>) -> bool {
		if self.clientList.read().unwrap().contains(&clientID) {return true;}
		return false;
	}
}

