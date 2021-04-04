#![allow(unused_imports)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::collections::HashMap;
use std::str;
use std::thread;
use std::thread::JoinHandle;
use std::sync::*;
use std::convert::TryInto;

use rand_core::{RngCore, OsRng};

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

use pss::*;
use pss::ModPow;

mod sockets;
pub mod worker;
pub mod param;
mod tests;
use sockets::*;
use param::*;
use worker::*;
use worker::ServerError;
use worker::WorkerError;
use tests::*;


#[derive(Debug)]
pub struct Profile {
	veriKey: VerifyKey,
	publicKey:  Vec<u8>,
	hasShared: bool,
}

pub struct Server {
	STATE: RwLock<usize>,	//readWrite Lock
	MAX: RwLock<usize>,
	V: usize,											// Vector size
	D: usize,											// Dropouts
	T: usize,											// Corruptions
	sessTime: usize,									// Time allowed for each state
	malFg: bool,
	param: RwLock<Param>,
	clientList: RwLock<Vec<Vec<u8>>>,					// array of ID
	clientProfiles: RwLock<HashMap<Vec<u8>, Profile>>,	// key = ID, value = Profile
	correctionVecs: Mutex<Vec<Vec<Vec<u64>>>>,
	shares: Mutex<Vec<Vec<u64>>>,
}


impl Server {

	pub fn new(maxClients: usize, 
		vectorSize: usize, dropouts: usize, sessionTime: usize, 
		corruption: usize, malicious: bool, mut param: Param) -> Server {
		Server {
			STATE: RwLock::new(1usize),
			MAX: RwLock::new(maxClients),
			V: vectorSize,
			D: dropouts,
			T: corruption,
			sessTime: sessionTime,
			malFg: malicious,
			param: RwLock::new(param),
			clientList: RwLock::new(Vec::new()),
			clientProfiles: RwLock::new(HashMap::<Vec<u8>, Profile>::new()),
			correctionVecs: Mutex::new(Vec::new()),
			/*	8 Tests in total:
				Degree test, Input Bit test, Quadratic test, Input bound test, 
				L2-norm sum test, L2-norm bit test, L2-norm bound test
			*/
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

		let timesUp = Arc::new(RwLock::new(false));
		let tu = timesUp.clone();
	    let (timerTx, timerRx) = mpsc::channel();
	    let timer = thread::spawn(move || {
	        match timer_task(timerRx, tu) {
	            Ok(_) => (),
	            Err(e) => println!("{:?}", e),
	        };
	    });

		timerTx.send(self.sessTime);

		let mut recvCnt = 0;
		let mut finalResult;
		let mut dropouts = Vec::new();
		loop {
			/* when finished client num exceed MAX
			initiate state change
			*/
			let tu =  (*timesUp.read().unwrap()).clone();
			let mut M = *self.MAX.read().unwrap();
			if tu || recvCnt >= M {
				println!("\n timesUp {:?}", tu);
				M = *self.MAX.write().unwrap();
				let mut stateGuard = self.STATE.write().unwrap();
				let mut list = match self.clientList.write() {
					Ok(mut guard) => guard,
					Err(_) => return Err(ServerError::MutexLockFail(0)),
				};
				let mut profiles = match self.clientProfiles.write() {
					Ok(mut guard) => guard,
					Err(_) => return Err(ServerError::MutexLockFail(0)),
				}; 
				let shares = match self.shares.lock() {
					Ok(guard) => guard,
					Err(_) => return Err(ServerError::MutexLockFail(0)),
				};
				let mut corrections = match self.correctionVecs.lock() {
					Ok(guard) => guard,
					Err(_) => return Err(ServerError::MutexLockFail(0)),
				};
				let mut param = match self.param.write() {
					Ok(guard) => guard,
					Err(_) => return Err(ServerError::MutexLockFail(0)),
				};
				let res = match *stateGuard {
					1 => {
						publish_vecs(
							&publisher, 
							format_clientData(&mut *profiles, &mut *list, "veriKey").unwrap(), 
							"HS");
						M = list.len();
						timerTx.send(self.sessTime)
					},
					2 => {
						/* Dropouts handled in format_clientData
						   removed from *list and *profiles if pk not found
						*/
						publish_vecs(
							&publisher, 
							format_clientData(&mut *profiles, &mut *list, "publicKey").unwrap(), 
							"KE");
						M = list.len();
						let sharingParams = match self.malFg {
							true => param.calculate_semi_honest(list.len(), self.V, self.D),
							false => param.calculate_malicious(list.len(), self.V, self.D, self.T),
						};
						println!("sharingParams {:?}", sharingParams);
						let mut spBytes = Vec::new();
						for sp in sharingParams {
							spBytes.extend(sp.to_le_bytes().to_vec());
						}
						publish(&publisher, spBytes, "IS");
						timerTx.send(self.sessTime)
					},
					3 => {
						/* Check dropouts from IS
						We don't remove anyone cuz resizing array is slow
						msg = [[dorpouts], [degree test], [Input Bit test], ....]
						*/
					   	let mut msg = vec![Vec::new(); 2];					//TODO: more tests to come....
						let mut new_dropouts = Vec::new();
						for (i, c) in list.iter().enumerate() {
							if !profiles.get(c).unwrap().hasShared {
								new_dropouts.push(i); 
								msg[0].extend((i as u64).to_le_bytes().to_vec());
							}
						}
						dropouts.extend(new_dropouts.clone());
					   	M = list.len();
						/* DegreeTest: 
						len = 5 sections * B blocks per sections
						*/
						let totalL =  5 * (self.V / param.L);
						for i in 0..totalL {
							msg[1].extend(OsRng.next_u64().to_le_bytes().to_vec());
						}
						/* M is updated 
						Corrections only contains the clients didn't dropout
						*/
						println!("IS dropouts {:?}, EC params len {:?}", new_dropouts, msg[1].len()/8);
						*corrections = vec![vec![vec![0; M]; M]; 7];		//TODO: more tests to come....
						publish_vecs(&publisher, msg, "EC");
						timerTx.send(self.sessTime)
					},
					4 => {
						/* Check dropouts from EC
						msg = [clients who dropouts or fail tests]
						*/
						let mut new_dropouts = Vec::new();
						for (i, c) in list.iter().enumerate() {
							// didn't recv corrections from Ci
							if corrections[0][0][i] == 0 {
								new_dropouts.push(i); 
							}
						}
					   	let mut msg = Vec::new();					
						for i in 0..M {
							if new_dropouts.contains(&i) { continue; }
							if !degree_test(&corrections[0][i]) { 			//When seeing 0, skip
								msg.extend(&(i as u64).to_le_bytes()); 
							}
							// TODO: other tests...
						}
						println!("EC dropouts & fail {:?}", msg.len());
						dropouts.extend(new_dropouts);
						publish(&publisher, msg, "AG");
						timerTx.send(self.sessTime)
					},
					5 => { 
						println!("recv Aggregated Shares {:?} {}", shares.len(), shares[0].len());
						finalResult = self.reconstruction(&shares, &dropouts, &param, M);
						break;
						Ok(())
					},
					_ => Ok(()),
				};
				match res {
					Ok(_) => {
						*stateGuard += 1;
						println!("Server: STATE change from {:?} to {:?}", *stateGuard-1, *stateGuard);
						recvCnt = 0;
						match timesUp.write() {
							Ok(mut guard) => *guard = false,
							Err(_) => return Err(ServerError::MutexLockFail(0)),
						}
					},
					Err(_) => return Err(ServerError::ThreadSenderFail(0)),
				};
			}

			match threadReciever.try_recv() {
				Ok(notification) => {
					/* worker thread send stateNum 
					when finish processing one client
					*/
					let mut stateGuard = self.STATE.write().unwrap();
					println!("Server mpsc recieved notification {:?}, cnt {}", notification, recvCnt+1);
					if notification == *stateGuard {
						recvCnt += 1;
					} 
				},
				Err(_) => continue,
			}
		}
		println!("finalResult {:?}", finalResult);
		return Ok(0)
	}

	pub fn worker_task(&self, worker: Worker)-> Result<usize, ServerError> {
		loop {
			let clientID = take_id(&worker.dealer);

			println!("{} Taken {:?}", 
				worker.ID, 
				String::from_utf8(clientID.clone()).unwrap());

			let msg = recv(&worker.dealer);

			let res = match *(self.STATE.read().unwrap()) {
				1 => self.handshake(&worker, clientID, msg),
				2 => self.key_exchange(&worker, clientID, msg),
				3 => self.input_sharing(&worker, clientID, msg),
				4 => self.error_correction(&worker, clientID, msg),
				5 => self.shares_collection(&worker, clientID, msg),
				_ => Err(WorkerError::UnknownState(0))
			};
			//res.unwrap();
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
					return Err(WorkerError::MaxClientExceed(1));
				}
				if guard.len() == *self.MAX.read().unwrap() {
		            send(&worker.dealer, "Error: Reached maximun client number.", &clientID);
					return Err(WorkerError::MaxClientExceed(1));	
				}
				guard.push(clientID.clone());
			},
			Err(_) => return Err(WorkerError::MutexLockFail(1)),
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
			Err(guard) => return Err(WorkerError::MutexLockFail(1)),
		};
		worker.threadSender.send(1);
		println!("handshaked with {:?}", std::str::from_utf8(&clientID).unwrap());
		return Ok(1);
	}

	fn key_exchange(&self, 
		worker: &Worker, clientID: Vec<u8>, msg: RecvType) -> Result<usize, WorkerError> {
	/*
		Make sure client exist
		Parse msg
	*/
		if !self.check_exist(&clientID) {
			send(&worker.dealer,"Error: Your profile not found", &clientID);
			return Err(WorkerError::ClientNotFound(1))
		}
		let publicKey;
		let singedPublicKey;
		match msg {
			RecvType::matrix(m) => {
				if m.len() != 2 {
					send(&worker.dealer, 
						"Please send with format: [publicKey, Enc(publicKey)]", &clientID);
					return Err(WorkerError::UnexpectedFormat(3))
				}
				publicKey = m[0].clone();
				singedPublicKey = Signature::from_bytes(&m[1]).unwrap();
			},
			_ => {
				send(&worker.dealer, 
					"Please send with ormat: [publicKey, Enc(publicKey, veriKey)]", &clientID);
				return Err(WorkerError::UnexpectedFormat(3))
			},
		}
	/*
		Clone veriKey
		Verify signature
		Sotre DH pk in *profiles
	*/
		let veriKey = match self.clientProfiles.read() {
			Ok(mut guard) => guard.get(&clientID).unwrap().veriKey.clone(),
			Err(_) => return Err(WorkerError::MutexLockFail(3)),
		};
		match veriKey.verify(&publicKey, &singedPublicKey) {
			Ok(_) => {
			 	match self.clientProfiles.write() {
					Ok(mut guard) => guard.get_mut(&clientID).unwrap().publicKey = publicKey.to_vec(),
					Err(_) => return Err(WorkerError::MutexLockFail(3)),
				};
		 		send(&worker.dealer, "Your publicKey has been save.", &clientID);
			},
			Err(_) => {
		 		send(&worker.dealer, "Error: Decryption Fail.", &clientID);
				return Err(WorkerError::DecryptionFail(3))
			},
		}
 		worker.threadSender.send(2);
 		println!("key_exchanged with {:?}", std::str::from_utf8(&clientID).unwrap());
		return Ok(2)
	}


	fn input_sharing(&self, 
		worker: &Worker, clientID: Vec<u8>, msg: RecvType) -> Result<usize, WorkerError> {
	/*
		Check client exiists
		Get the share and delivery target
	*/
		println!("{:?} in input_sharing", worker.ID);
		if !self.check_exist(&clientID) {
			send(&worker.dealer,"Error: Your profile not found", &clientID);
			return Err(WorkerError::ClientNotFound(3))
		}
		let M = self.MAX.read().unwrap().clone();
		let L = self.param.read().unwrap().L;
		let B = self.V / L;
		let shares = match msg {
			RecvType::matrix(m) => {
				if m.len() != M {		//TODO: only 5 sections
					send(&worker.dealer, "Please share with specified parameters.", &clientID);
					return Err(WorkerError::UnexpectedFormat(3)); 
				}
				m
			}, 
			_ => {
				send(&worker.dealer, "Please send your shares as matrix.", &clientID);
				return Err(WorkerError::UnexpectedFormat(3));
			},
		};
	/*
		For each outbounding share
		Send [senderPk, share]
	*/
		let listGuard = match self.clientList.read() {
			Ok(guard) => guard,
			Err(_) => return Err(WorkerError::MutexLockFail(0)),
		};
		let senderPk = match self.clientProfiles.read() {
			Ok(guard) => guard.get(&clientID).unwrap().publicKey.clone(),
			Err(_) => return Err(WorkerError::MutexLockFail(0)),
		}; 
				
		assert_eq!(shares.len(), listGuard.len());
		for i in 0..shares.len() {
			/* 
			attach senderPK so that reciever knows who is this from
			and which sharedKey to use
			*/
			let msg = vec![senderPk.clone(), shares[i].clone()];
			match send_vecs(&worker.dealer, msg, &listGuard[i]) {
				Ok(_) => continue,
				Err(_) => return Err(WorkerError::SharingFail(3)),
			};
			println!("share (len: {:?}) from {:?} to {:?}", 
				shares[i].len(), str::from_utf8(&clientID).unwrap(), str::from_utf8(&listGuard[i]).unwrap());
		}
		match self.clientProfiles.write() {
			Ok(mut guard) => guard.get_mut(&clientID).unwrap().hasShared = true,
			Err(_) => return Err(WorkerError::MutexLockFail(0)),
		}; 
		worker.threadSender.send(3);
		return Ok(3)
	}

	fn error_correction(&self, 
		worker: &Worker, clientID: Vec<u8>, msg: RecvType) -> Result<usize, WorkerError> {
	/*
		Check client exiists
	*/
		if !self.check_exist(&clientID) {
			send(&worker.dealer,"Error: Your profile not found", &clientID);
			return Err(WorkerError::ClientNotFound(4))
		}
		// M has been updated
		let M = self.MAX.read().unwrap().clone();
		let idx = self.clientList
				.read().unwrap()
				.iter().position(|s| s == &clientID)
				.unwrap();
		match msg {
			RecvType::matrix(m) => {
				if m.len() != M || m[0].len() != 7 * 8 {			//TODO: more tests to come....
					send(&worker.dealer, "Please send your degree test matrix. 
											Format: [[Degree test], [Input Bit test], [Quadratic test], [Input bound test], 
											[L2-norm sum test], [L2-norm bit test], [L2-norm bound test]]", &clientID);
					return Err(WorkerError::UnexpectedFormat(4))
				}
				/* correctionVecs Format:
										Test1			Test2				    Test7
				c1's collection 	[[c11... cm1]	 [[c11... cm1]  .....  [[c11... cm1]
				c2's collection  	[c12... cm2]	  [c12... cm2]  .....   [c11... cm2]
						....
				cm's collection  	[c1m... cmm]]	 [c1m... cmm]]  .....   [c1... cmm]]
				*/
				match self.correctionVecs.lock() {
					Ok(mut guard) => {
						for i in 0..M {
							let testsResults_ci = read_le_u64(&m[i]);
							for j in 0..7 {
								guard[j][i][idx] = testsResults_ci[j];
							}
						}
					},
					Err(_) => return Err(WorkerError::MutexLockFail(4)),
				};
			}, 
			_ => {
				send(&worker.dealer, "Please send your degree test matrix. 
					Format: [[Degree test], [Input Bit test], [Quadratic test], [Input bound test], 
					[L2-norm sum test], [L2-norm bit test], [L2-norm bound test]]", &clientID);
				return Err(WorkerError::UnexpectedFormat(4))
			},
		};
		println!("error_correction doen");
		worker.threadSender.send(4);
		return Ok(4);
	}

	fn shares_collection(&self, 
		worker: &Worker, clientID: Vec<u8>, msg: RecvType) -> Result<usize, WorkerError> {
	/*
		Check client exist
		Get shares & signature
	*/

	//?
		if !self.check_exist(&clientID) {
			send(&worker.dealer,"Error: Your profile not found", &clientID);
			return Err(WorkerError::ClientNotFound(5))
		}
		let msg = match msg {
			RecvType::matrix(m) => {
				if m.len() != 2 {
					send(&worker.dealer, 
						"Please send your shares with a signature. Format: [shares, Enc(shares)]", 
						&clientID);
					return Err(WorkerError::UnexpectedFormat(5))
				}
				m
			},
			_ => {
				send(&worker.dealer, 
					"Please send your shares key with a signature. Format: [shares, Enc(shares)]", 
					&clientID);
				return Err(WorkerError::UnexpectedFormat(5))
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
			Err(_) => return Err(WorkerError::MutexLockFail(5)),
		};
		match verifyResult {
			Ok(_) => {
				self.shares.lock().unwrap().push(read_le_u64(&msg[0]));
		 		send(&worker.dealer, 
		 			"Your aggregated shares has been save.", 
		 			&clientID);
		 		worker.threadSender.send(5);
		 		return Ok(5)
			},
			Err(_) => {
		 		send(&worker.dealer, "Error: Decryption Fail.", &clientID);
				return Err(WorkerError::DecryptionFail(5))
			},
		}		

	}


	fn reconstruction(&self, shares: &Vec<Vec<u64>>, dropouts: &Vec<usize>, param: &Param, M: usize) -> Result<Vec<u64>, WorkerError> {
		// Handles dropouts
		let B = shares[0].len();
		let N = shares.len();
		let P = param.P as u128;
		let R3 = param.useR3 as u128;
		let pss = PackedSecretSharing::new(
			P, param.useR2 as u128, R3, 
			param.useD2, param.useD3, self.V, param.L, N
		);
		//println!("reconstruction param {:?}", param);
		let mut sharesPoints = Vec::new();
		for i in 0..M {
			if !dropouts.contains(&i) {
		    	sharesPoints.push(R3.modpow((i+1) as u128, P) as u64);
			}
		}
		let ret = pss.reconstruct(&shares, sharesPoints.as_slice());
		println!("Reconstruction DONE");
		println!("constructed len {:?}", ret.len());
		return Ok(ret);
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

