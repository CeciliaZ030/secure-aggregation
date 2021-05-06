#![allow(unused_imports)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]
#![allow(unused_must_use)]

use std::collections::HashMap;
use std::thread;
use std::sync::*;
use std::time::Instant;

use rand_core::{RngCore, OsRng};

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
	S: usize,											// Input Bit Limit
	D: usize,											// Dropouts
	T: usize,											// Corruptions
	sessTime: usize,									// Time allowed for each state
	ISsessTime: usize,									// Time allowed for each state
	malFg: bool,
	param: RwLock<Param>,
	clientList: RwLock<Vec<Vec<u8>>>,					// array of ID
	clientProfiles: RwLock<HashMap<Vec<u8>, Profile>>,	// key = ID, value = Profile
	correctionVecs: Mutex<Vec<Vec<Vec<u64>>>>,
	shares: Mutex<Vec<Vec<u64>>>,
}


impl Server {

	pub fn new(maxClients: usize, 
		vectorSize: usize, inputBitLimit: usize,
		dropouts: usize, sessionTime: usize, ISsessTime: usize,
		corruption: usize, malicious: bool, mut param: Param) -> Server {
		println!("maxClients {:?} vectorSize {} dropouts {} sessionTime {} ISsessTime {} corruption {} malicious {}", 
			maxClients, vectorSize, dropouts, sessionTime, ISsessTime, corruption, malicious);
		Server {
			STATE: RwLock::new(1usize),
			MAX: RwLock::new(maxClients),
			V: vectorSize,
			S: inputBitLimit,
			D: dropouts,
			T: corruption,
			sessTime: sessionTime,
			ISsessTime: ISsessTime,
			malFg: malicious,
			param: RwLock::new(param),
			clientList: RwLock::new(Vec::new()),
			clientProfiles: RwLock::new(HashMap::<Vec<u8>, Profile>::new()),
			correctionVecs: Mutex::new(Vec::new()),
			shares: Mutex::new(Vec::new()),
		}
	}

	pub fn server_task(&self, 
		context: zmq::Context, ip: Option<&str>, port1: usize) -> Result<usize, ServerError>  {

		let frontend = context.socket(zmq::ROUTER).unwrap();
    	let backend = context.socket(zmq::DEALER).unwrap();

    	match ip {
    		Some(address) => {
    			println!("Reciever connecting to tcp://{}:{}", address, port1);
				assert!(frontend
					.bind(&format!("tcp://{}:{:?}", address, port1))
					.is_ok());
    		},
    		None => {
				println!("Reciever going default tcp://*:{}", port1);
				assert!(frontend
					.bind(&format!("tcp://*:{:?}", port1))
					.is_ok());
    		},
    	}
		assert!(backend
			.bind("inproc://backend")
			.is_ok());

		zmq::proxy(&frontend, &backend);
		return Ok(0)
	}



	pub fn state_task(&self, 
		context: zmq::Context, ip: Option<&str>, port2: usize, threadReciever: mpsc::Receiver<usize>) -> Result<usize, ServerError> {

		let publisher = context.socket(zmq::PUB).unwrap();
        publisher.set_sndhwm(1_100_000).expect("failed setting hwm");
    	match ip {
    		Some(address) => {
				println!("Publisher connecting to tcp://{}:{:?}", address, port2);
				assert!(publisher
					.bind(&format!("tcp://{}:{:?}", address, port2))
					.is_ok());
    		},
    		None => {
				println!("Publisher going default tcp://*:{:?}", port2);
				assert!(publisher
					.bind(&format!("tcp://*:{:?}", port2))
					.is_ok());
    		},
    	}
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
		let mut BENCH_TIMER = Instant::now();
		loop {
			/* when finished client num exceed MAX
			initiate state change
			*/
			let tu =  (*timesUp.read().unwrap()).clone();
			let mut M = *self.MAX.read().unwrap();
			if tu || recvCnt >= M {
				// println!("\n timesUp {:?}", tu);
				M = *self.MAX.write().unwrap();
				let mut stateGuard = self.STATE.write().unwrap();
				println!("- State {} elapse {:?}ms", *stateGuard, BENCH_TIMER.elapsed().as_millis());
				BENCH_TIMER = Instant::now();
				if *stateGuard == 6 {
					println!("Server shutting down");
					break;
				}

				let mut list = match self.clientList.write() {
					Ok(mut guard) => guard,
					Err(_) => return Err(ServerError::MutexLockFail(0)),
				};
				let mut profiles = match self.clientProfiles.write() {
					Ok(mut guard) => guard,
					Err(_) => return Err(ServerError::MutexLockFail(0)),
				}; 
				let mut shares = match self.shares.lock() {
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
						if recvCnt == 0 {panic!("No one joins!");}
						publish_vecs(
							&publisher, 
							format_clientData(&mut *profiles, &mut *list, "veriKey").unwrap(), 
							"HS");
						M = list.len();
						// for (i, c) in (&*list).iter().enumerate() {
						// 	print!("{}: {:?}, ",i, str::from_utf8(c).unwrap());
						// }
						timerTx.send(self.sessTime)
					},
					2 => {
						/* Dropouts handled in format_clientData
						   removed from *list and *profiles if pk not found
						   Not using recording dropouts before IS begins
						*/
						println!("cp {:?}", *list);
						publish_vecs(
							&publisher, 
							format_clientData(&mut *profiles, &mut *list,
							 "publicKey").unwrap(), 
							"KE");
						println!("cp2");
						M = list.len();
						let sharingParams = match self.malFg {
							false => param.calculate_semi_honest(M, self.V, self.S, self.D),
							true => param.calculate_malicious(M, self.V, self.S, self.D, self.T),
						};
						println!("sharingParams {:?}", sharingParams);
						println!("L {:?}", sharingParams[5]);
						let mut spBytes = Vec::new();
						for sp in sharingParams {
							spBytes.extend(sp.to_le_bytes().to_vec());
						}
						*shares = vec![Vec::new(); M];
						publish(&publisher, spBytes, "IS");
						timerTx.send(self.ISsessTime)
					},
					3 => {
						/* Check dropouts from IS
						We don't remove anyone cuz resizing array is slow
						msg = [[dorpouts], [degree test], [Input Bit test], ....]
						*/
					   	let mut msg = vec![Vec::new(); 9];					//TODO: more tests to come....
						let mut new_dropouts = Vec::new();
						for (i, c) in list.iter().enumerate() {
							if !profiles.get(c).unwrap().hasShared {
								new_dropouts.push(i); 
								msg[0].extend(&(i.to_le_bytes()));
							}
						}
						dropouts.extend(new_dropouts.clone());

						let L = param.L;
						let B = self.V / L;
						let S = self.S;
						println!("{}, {}, {}", L, B, S);
                        // maximun bits length of ySum
						let Y = (
							((2f32*(S as f32) + (self.V as f32).log2().ceil())/
							(L as f32)).ceil()*
							(L as f32)) as usize;
                        println!("{}", Y);
						// Degree Test
						for i in 0..(2*self.V + L + Y + L*S*B + 3*L)/L {
							msg[1].extend(&(OsRng.next_u64() % param.P).to_le_bytes());
						}
					
						// Input Bit Test
						for i in 0..B*S {
							msg[2].extend(&(OsRng.next_u64() % param.P).to_le_bytes());
						}	
						
						// Quadratic Test
						for i in 0..B {
							msg[3].extend(&(OsRng.next_u64() % param.P).to_le_bytes());
						}
						
						// Input bound test
						for i in 0..B {
							msg[4].extend(&(OsRng.next_u64() % param.P).to_le_bytes());
						}

						// L2-norm sum test
						msg[5].extend(&(OsRng.next_u64() % param.P).to_le_bytes());

						// L2-norm bit test
						for i in 0..Y/L {
							msg[6].extend(&(OsRng.next_u64() % param.P).to_le_bytes());
						}
						// L2-norm bound test
						for i in 0..Y/L {
							msg[7].extend(&(OsRng.next_u64() % param.P).to_le_bytes());
						}				
                        println!("Y {}", Y);
						let mut twoPowers = Vec::<u64>::new();
						let bit_num = (2f32*(S as f32) + (self.V as f32).log2().ceil()) as usize;
						for i in 0..bit_num {
							twoPowers.push(2u64.pow(i as u32));
						}
						for i in bit_num..Y {
							twoPowers.push(0u64);
						}
                        let mut pss = PackedSecretSharing::new(
							param.P as u128, param.useR2 as u128, param.useR3 as u128, 
							param.useD2, param.useD3, Y, L, M
						);
						let twoPowers_shares = pss.share(&twoPowers);
                        for share in twoPowers_shares {
							msg[8].extend(write_u64_le_u8(share.as_slice()));
						}
				
						/* M is updated 
						Corrections only contains the clients didn't dropout
						*/
						println!("IS dropouts {:?}, EC params {:?}", new_dropouts, msg.len());
						*corrections = vec![vec![Vec::new(); M]; M];
						publish_vecs(&publisher, msg, "EC");
						timerTx.send(self.sessTime)
					},
					4 => {
						/* Check dropouts from EC
							msg = [clients who dropouts or fail tests]
						*/
						println!("EC dropouts {:?}", dropouts);
						let mut pss = PackedSecretSharing::new(
                                param.P as u128, param.useR2 as u128, param.useR3 as u128,
                                param.useD2, param.useD3, 3*param.L, param.L, M
                        );
                        let mut ThreadPool = Vec::new();
                        for i in 0..M {
							let mut j = 0;
							while j < M && corrections[i][j].len() == 0 {
								// if row_i is empty then party_i must dropout from last round
								j += 1;
								if j == M { 
									dropouts.push(i); 
									continue;
								}
							}
                            let corrections_ = (corrections[i]).clone();
							let param_ = (*param).clone();
							let child = thread::spawn(move || {
								test_suit(&corrections_, &param_)
							});
							ThreadPool.push(child);
                            //if !test_suit(&(*corrections)[i], &param, &mut dropouts, &pss) {
							//		dropouts.push(i);
							//}

						}
                        let mut cnt = 0;
						for t in ThreadPool {
							let is_pass = t.join().unwrap();
							if !is_pass {
								dropouts.push(cnt);
							}
							cnt += 1;
						}
					   	let mut msg = Vec::new();
						msg.extend(write_usize_le_u8(dropouts.as_slice()));
						publish(&publisher, msg, "AG");
						timerTx.send(self.sessTime)
					},
					5 => { 
						finalResult = self.reconstruction(&shares, &dropouts, &param, M);
						timerTx.send(1)
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
					//println!("Server mpsc recieved notification {:?}, cnt {}", notification, recvCnt+1);
					if notification == *stateGuard {
						recvCnt += 1;
					} 
				},
				Err(_) => continue,
			}
		}
		return Ok(0)
	}

	pub fn worker_task(&self, worker: Worker)-> Result<usize, ServerError> {
		let mut EC_skip = false;
		let mut i = 0;
		loop {
			let clientID = take_id(&worker.dealer);
			// println!("worker loop {:?}", i);
			// println!("{} Taken {:?}", 
				// worker.ID, 
				// String::from_utf8(clientID.clone()).unwrap());

			let msg = recv(&worker.dealer);

			let res = match *(self.STATE.read().unwrap()) {
				1 => self.handshake(&worker, clientID, msg),
				2 => self.key_exchange(&worker, clientID, msg),
				3 => self.input_sharing(&worker, clientID, msg),
				4 => self.error_correction(&worker, clientID, msg),
				5 => self.shares_collection(&worker, clientID, msg),
				_ => Err(WorkerError::UnknownState(0)),
			};
			i += 1;
			match res {
				Ok(_) => continue,
				Err(e) => println!("{:?}", e),
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
		//println!("handshaked with {:?}", std::str::from_utf8(&clientID).unwrap());
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
 		//println!("key_exchanged with {:?}", std::str::from_utf8(&clientID).unwrap());
		return Ok(2)
	}


	fn input_sharing(&self, 
		worker: &Worker, clientID: Vec<u8>, msg: RecvType) -> Result<usize, WorkerError> {
	/*
		Check client exiists
		Get the share and delivery target
	*/
		//println!("{:?} in input_sharing", worker.ID);
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
			//println!("{:?}", msg);
			match send_vecs(&worker.dealer, msg, &listGuard[i]) {
				Ok(_) => {
					//println!("share (len: {:?}) from {:?} to {:?}", 
					//	shares[i].len(), str::from_utf8(&clientID).unwrap(), str::from_utf8(&listGuard[i]).unwrap());
				},
				Err(_) => return Err(WorkerError::SharingFail(3)),
			};
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
		//println!("{:?} error_correction", worker.ID);
		if !self.check_exist(&clientID) {
			send(&worker.dealer,"Error: Your profile not found", &clientID);
			return Err(WorkerError::ClientNotFound(4))
		}
		// M stays the same
		let M = self.MAX.read().unwrap().clone();
		let idx = self.clientList
				.read().unwrap()
				.iter().position(|s| s == &clientID)
				.unwrap();
		match msg {
			RecvType::matrix(m) => {
				// client_i dropouts then row_i is empty
				// 3 tests results * 8 bytes per tests result
				if m.len() != M || (m[0].len() != 3 * 8 && m[0].len() != 0) {
					send(&worker.dealer, "Please send your degree test matrix. 
											Format: [[Degree test], [Input Bit test], [Quadratic test], [Input bound test], 
											[L2-norm sum test], [L2-norm bit test], [L2-norm bound test]]", &clientID);
					return Err(WorkerError::UnexpectedFormat(4))
				}
		match self.correctionVecs.lock() {
					Ok(mut guard) => {
						for i in 0..M {
							let testsVecs_ci = read_le_u64(&m[i]);
							guard[i][idx] = testsVecs_ci;
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
		let idx = self.clientList
				.read().unwrap()
				.iter().position(|s| s == &clientID)
				.unwrap();
		match verifyResult {
			Ok(_) => {
				let mut shares = self.shares.lock().unwrap();
				shares[idx] = read_le_u64(&msg[0]);
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
	/*
		Perform PSS reconstruction
		shares contains empty entries:
		 [s0, s1, _, s3, ...., _, ..., sM] which are the dropouts
		construct compact value points & eval points
		 [s0, s1, s3, ..., sM]
		 [x0, x1, x3, ..., xM]
		then do Lagrange

	*/
		let B = shares[0].len();
		let N = shares.len();
		let P = param.P as u128;
		let R3 = param.useR3 as u128;
		let R2 = param.useR2 as u128;
		let pss = PackedSecretSharing::new(
			P, R2, R3, 
			param.useD2, param.useD3, self.V, param.L, N
		);
		let mut sharesPoints = Vec::new();
		let mut shares_remove_empty = Vec::new();
		for i in 0..M {
			if shares[i].len() == 0 {
		    	continue;
			}
			sharesPoints.push(R3.modpow((i+1) as u128, P) as u64);
			shares_remove_empty.push(shares[i].clone());
		}
		println!("shares_remove_empty {:?}, sharesPoints {}", shares_remove_empty.len(), sharesPoints.len());
		let ret = pss.reconstruct(&shares_remove_empty, sharesPoints.as_slice());
		println!("Reconstruction DONE {:?}", ret);
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

