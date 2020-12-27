use std::str;
use std::result::Result;
use std::thread;
use std::thread::*;
use std::sync::*;

use zmq::SNDMORE;
use zmq::Message;

use rand_core::{OsRng};
use p256::{
	EncodedPoint,
	ecdh::{EphemeralSecret, SharedSecret},
    ecdsa::{SigningKey, Signature, signature::Signer, VerifyKey},
};
use aes_gcm::Aes256Gcm; // Or `Aes128Gcm`
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, NewAead};

use packed_secret_sharing::packed::*;

mod sockets;
use sockets::*;

#[derive(Debug)]
pub enum ClientError {
	SendFailure(usize),
	UnexpectedRecv(RecvType),
	EncryptionError(usize),
	MutexLockFail(usize),
}

pub struct Client{

	pub context: zmq::Context,
	pub sender: zmq::Socket,
	pub subscriber: zmq::Socket,
	subPort: String,

	pub ID: String,						// Unique ID that is field element

	signKey: SigningKey,				// Authentification
	veriKey: VerifyKey,

	privateKey: EphemeralSecret,		// ECDH
	publicKey: EncodedPoint,

	clientVerikeys: Vec<Vec<u8>>,
	shareKeys: Vec<(Vec<u8>, Vec<u8>)>, // Array of (pubKey of peer, shareKey with peer)

	workerHandle: JoinHandle<()>,
	sendQueue: Arc<RwLock<Vec<RecvType>>>,
	recvQueue: Arc<RwLock<Vec<RecvType>>>,
	recvPtr: usize,
}

pub struct Worker {
	pub socket: zmq::Socket,
	sendQueue: Arc<RwLock<Vec<RecvType>>>,
	recvQueue: Arc<RwLock<Vec<RecvType>>>,
}

impl Worker {

	pub fn new(ID: &'static str, 
		context: zmq::Context, port: &'static str, 
		sendQueue: Arc<RwLock<Vec<RecvType>>>, recvQueue: Arc<RwLock<Vec<RecvType>>>) -> Worker {
		
		let socket = context.socket(zmq::DEALER).unwrap();
		let mut addr1: String = "tcp://localhost:".to_owned();
		addr1.push_str(port);
		socket.set_identity(ID.as_bytes());
		assert!(socket.connect(&addr1).is_ok());
		Worker {
			socket: socket,
			sendQueue: sendQueue,
			recvQueue: recvQueue
		}
	}

	pub fn start(&self) -> Result<usize, ClientError> {
		loop {
			// Clear sendQueues
			let mut sqGuard;
			match self.sendQueue.read() {
				Ok(guard) => sqGuard = guard,
				Err(_) => return Err(ClientError::MutexLockFail(0)),
			}
			if !sqGuard.is_empty() {
				let mut sqGuard;
				match self.sendQueue.write() {
					Ok(guard) => sqGuard  = guard,
					Err(_) => return Err(ClientError::MutexLockFail(0)),
				}
				for msg in &*sqGuard {
					match msg {
						RecvType::bytes(b) => send(&self.socket, b),
						RecvType::string(s) => send(&self.socket, s.as_str()),
						RecvType::matrix(m) => send_vecs(&self.socket, m),
					};
				}
				*sqGuard = Vec::new();
			}
			// Clear recv socket
			let mut item = [self.socket.as_poll_item(zmq::POLLIN)];
	        zmq::poll(&mut item, 2).unwrap();
	        if item[0].is_readable() {
	        	let msg = recv(&self.socket);
				match self.recvQueue.write() {
					Ok(mut rqGuard) => (*rqGuard).push(msg),
					Err(_) => return Err(ClientError::MutexLockFail(0)),
				};
	        }
		}
		return Ok(0)
	}
}


impl Client{
	
	pub fn new(ID: &'static str, 
		context: zmq::Context, port1: &'static str, port2: &str) -> Client{

		let sender = context.socket(zmq::DEALER).unwrap();
		let subscriber = context.socket(zmq::SUB).unwrap();

		let mut addr1: String = "tcp://localhost:".to_owned();
		let mut addr2: String = "tcp://localhost:".to_owned();

		addr1.push_str(port1);
		addr2.push_str(port2);
		sender.set_identity(ID.as_bytes());
		assert!(sender.connect(&addr1).is_ok());
		assert!(subscriber.connect(&addr2).is_ok());

		let sendQueue = Arc::new(RwLock::new(Vec::<RecvType>::new()));
		let recvQueue = Arc::new(RwLock::new(Vec::<RecvType>::new()));

		let ctx = context.clone();
		let sq = sendQueue.clone();
		let rq = recvQueue.clone();
	    let workerThread = thread::spawn(move || {
			let worker = Worker::new(ID, ctx, port1, sq, rq);
			worker.start();
	    });

		let signKey = SigningKey::random(&mut OsRng);
		let veriKey = VerifyKey::from(&signKey);
		let privateKey = EphemeralSecret::random(&mut OsRng);
		let publicKey = EncodedPoint::from(&privateKey);

		Client {

			context: context,
			sender: sender,
			subscriber: subscriber,
			subPort: addr2,

			ID: ID.to_string(),

			signKey: signKey,
			veriKey: veriKey,

			privateKey: privateKey,
			publicKey: publicKey,

			clientVerikeys: Vec::<Vec<u8>>::new(),
			shareKeys: Vec::new(),

			workerHandle: workerThread,
			sendQueue: sendQueue,
			recvQueue: recvQueue,
			recvPtr: 0usize,
		}
	}

	pub fn handshake(&mut self) -> Result<usize, ClientError> {

		match self.sendQueue.write() {
			Ok(mut guard) => guard.push(
					RecvType::string(format!("Hello, I'm {}", self.ID))
				),
			Err(_) => return Err(ClientError::MutexLockFail(0)),
		}
		let sk = match self.recvQueue.read() {
			Ok(guard) => {
				assert!(guard.len() >= self.recvPtr);
				match &guard[self.recvPtr] {
					RecvType::bytes(b) => b,
					_ => return Err(ClientError::UnexpectedRecv(guard[self.recvPtr].clone())),
				};		
			},
			Err(_) => return Err(ClientError::MutexLockFail(0)),
		};

		let recvRes = recv(&self.sender);
		let sk = match recvRes {
			RecvType::bytes(b) => b,
			_ => return Err(ClientError::UnexpectedRecv(recvRes)),
		};

		match SigningKey::new(&sk) {
			Ok(k) => self.signKey = k,
			Err(_) => return Err(ClientError::EncryptionError(0)),
		};
		self.veriKey = VerifyKey::from(&self.signKey);

		let waitRes = self.wait_state_change("HS");
		match waitRes {
			RecvType::matrix(m) => {
				println!("Recieved other's vk: \n {:?}", &m);
				self.clientVerikeys = m;
			},
			_ => return Err(ClientError::UnexpectedRecv(waitRes)),
		}
		println!("OK from handshake");
		return Ok(0)
	}


	pub fn key_exchange(&mut self) -> Result<usize, ClientError> {

		let publicKeyVec = self.publicKey.to_bytes();
		let signedPublicKey: Signature = self.signKey.sign(&publicKeyVec);

		match self.sendQueue.write() {
			Ok(mut guard) => guard.push(RecvType::matrix(
					vec![publicKeyVec.to_vec(),
					signedPublicKey.as_ref().to_vec()]
				)),
			Err(_) => return Err(ClientError::MutexLockFail(1)),
		}

		match self.recvQueue.read() {
			Ok(guard) => {
				assert!(guard.len() >= self.recvPtr);
				match &guard[self.recvPtr] {
					RecvType::string(s) => {
						println!("{:?}", s);
						//server says Ok
					},
					_ => return Err(ClientError::UnexpectedRecv(guard[self.recvPtr].clone())),
				};			
			},
			Err(_) => return Err(ClientError::MutexLockFail(1)),
		}

		let waitRes = self.wait_state_change("KE");
		let publicKeys = match waitRes {
			RecvType::matrix(m) => m,
			_ => return Err(ClientError::UnexpectedRecv(waitRes)),
		};
		println!("Recieved other's pk: \n {:?}", &publicKeys);
		for pk in publicKeys {
			let shared = self.privateKey
							 .diffie_hellman(
							 	&EncodedPoint::from_bytes(&pk).unwrap()
							 );
			match shared {
				Ok(s) => {
					/* May run into your own public key 
					creating share key with yourself
					It's okay, you can decrypt your share as if you're a peer
					*/
					self.shareKeys.push((pk, s.as_bytes().to_vec()));
				},
				Err(_) => return Err(ClientError::EncryptionError(1)),
			};
		}
		println!("OK from key_exchange");
		return Ok(1) 
	}

	pub fn input_sharing(&mut self, input: &Vec<u64>) -> Result<usize, ClientError> {

		let waitRes = self.after_state_change("IS");
		let sharingParams = match waitRes {
			RecvType::matrix(m) => {
				assert_eq!(m.len(), 7);
				m
			},
			_ => return Err(ClientError::UnexpectedRecv(waitRes)),
		};

		let degreeTwo = u128::from_le_bytes(bytesToArr(&sharingParams[0])) as usize;
		let degreeThree = u128::from_le_bytes(bytesToArr(&sharingParams[1])) as usize;
		let blockLength = u128::from_le_bytes(bytesToArr(&sharingParams[2])) as usize;
		let numCorrupted = u128::from_le_bytes(bytesToArr(&sharingParams[3]));
		let prime = u128::from_le_bytes(bytesToArr(&sharingParams[4]));
		let rootTwo = u128::from_le_bytes(bytesToArr(&sharingParams[5]));
		let rootThree = u128::from_le_bytes(bytesToArr(&sharingParams[6]));
		let clientNum = self.shareKeys.len();

		let mut pss = PackedSecretSharing::new(prime, 
											rootTwo, rootThree, 
											degreeTwo, degreeThree, 
											blockLength, clientNum);
		let mut resultMatrix = vec![vec![0u8; 0]; clientNum];

		println!("computing shares with param d2: {:?}, d3: {}, block: {}, clientNum {}", degreeTwo, degreeThree, blockLength, clientNum);
		
		for i in 0..input.len()/blockLength {
			let shares = pss.share_u64(
				&input[blockLength*i..blockLength+blockLength*i]	// on heap
			);
			for j in 0..clientNum {
				resultMatrix[j].extend((shares[j] as u64).to_le_bytes().to_vec())
			}
		}
		println!("raw shares (vec len {})..", resultMatrix[0].len());
		for (i, key) in self.shareKeys.iter().enumerate() {
			
			// Encrypt with the share key
			let k = GenericArray::from_slice(&key.1);
			let cipher = Aes256Gcm::new(k);
			let nonce = GenericArray::from_slice(b"unique nonce");
			let encryptedShares = cipher.encrypt(nonce, resultMatrix[i].as_slice())
		    					   		.expect("encryption failure!");
			println!("encryptedShares len {:?}", encryptedShares.len());
			
			/* Prepend the pubKey of this peer 
		    so that server know who to forward
		    */
		    let mut msg = Vec::new();
		    msg.push(key.0.clone());
		    msg.push(encryptedShares);

			match self.sendQueue.write() {
				Ok(mut guard) => guard.push(RecvType::matrix(msg)),
				Err(_) => return Err(ClientError::MutexLockFail(1)),
			};
		}

		// recieving shares 
		let mut shares = Vec::new();
		match self.recvQueue.read() {
			Ok(guard) => {
				let mut recvSoFar = 0;
				loop {
					let i = 0;
					while(self.recvPtr + i < guard.len()){
						match &guard[self.recvPtr + i] {
							RecvType::matrix(m) => shares.push(m.clone()),
							_ => return Err(ClientError::UnexpectedRecv(guard[self.recvPtr + i].clone())),
						};
					}
					self.recvPtr += i;
					recvSoFar += i;
					if recvSoFar == clientNum {
						break;
					}
				}
			},
			Err(_) => return Err(ClientError::MutexLockFail(1)),
		}

		return Ok(2)

	}

	pub fn wait_state_change(&self, curState: &str) -> RecvType {

		let subscriber = self.context.socket(zmq::SUB).unwrap();
		assert!(subscriber.connect(&self.subPort).is_ok());

		println!("waiting for {} ....", curState);
		subscriber.set_subscribe(curState.as_bytes()).unwrap();
		
		loop {
			let mut item = [subscriber.as_poll_item(zmq::POLLIN)];
	        zmq::poll(&mut item, 2).unwrap();
	        if item[0].is_readable() {
	        	println!("state change {}\n", curState);
	        	return recv_broadcast(&subscriber)
	        }
		}
	}

	pub fn after_state_change(&self, curState: &str) -> RecvType {

		let subscriber = self.context.socket(zmq::SUB).unwrap();
		assert!(subscriber.connect(&self.subPort).is_ok());

		println!("getting info for {} ....", curState);
		subscriber.set_subscribe(curState.as_bytes()).unwrap();
		
		loop {
			let mut item = [subscriber.as_poll_item(zmq::POLLIN)];
	        zmq::poll(&mut item, 2).unwrap();
	        if item[0].is_readable() {
	        	return recv_broadcast(&subscriber)
	        }
		}
	}
}


fn bytesToArr(v: &Vec<u8>) -> [u8; 16] {
	let mut res = [0u8; 16];
	for (i, t) in v.iter().enumerate() {
		res[i] = t.clone();
	}
	return res
}