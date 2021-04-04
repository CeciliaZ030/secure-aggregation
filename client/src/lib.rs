use std::str;
use std::collections::HashMap;
use std::time::Duration;
use std::convert::TryInto;
use std::sync::*;
use std::thread;
use std::thread::sleep;

use zmq::SNDMORE;
use zmq::Message;

use rand_core::{RngCore, OsRng};
use signature::Signature as _;
use p256::{
	NistP256,
	EncodedPoint,
	ecdh::{EphemeralSecret, SharedSecret},
    ecdsa::{
    	SigningKey, Signature, signature::Signer, 
    	VerifyKey, signature::Verifier
    }
};

use aes_gcm::Aes256Gcm; // Or `Aes128Gcm`
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, NewAead};

use pss::*;

mod sockets;
use sockets::*;

#[derive(Debug)]
pub enum ClientError {
	SendFailure(usize),
	UnexpectedRecv(RecvType),
	EncryptionError(usize),
	MutexLockFail(usize),
	UnidentifiedShare(usize),
}


#[derive(Debug, Clone, Copy)]
pub struct Param {
	P: u128,
	R2: u128,
	R3: u128,
	D2: usize,
	D3: usize,
	L: usize,
	remainder: usize,
}


pub struct Client{

	pub ID: String,							//Unique ID that is field element
	context: zmq::Context,
	sender: zmq::Socket,

	subThread :thread::JoinHandle<Result<usize, ClientError>>,
	buffer: Arc<RwLock<HashMap<Vec<u8>, RecvType>>>,

	signKey: SigningKey,					//Authentification
	veriKey: VerifyKey,

	privateKey: EphemeralSecret,			//ECDH
	publicKey: EncodedPoint,

	clientVerikeys: Vec<Vec<u8>>,
	shareKeys: HashMap<Vec<u8>, Vec<u8>>, 	// {key: pubKey, value: shareKey}
	shareOrder:  Vec<Vec<u8>>,				/* [pk_c1, pk_c2, ....] 
											all clients assign shares in this order
											*/

	vectorSize: usize,
	param: Option<Param>,
	shares: Vec<Vec<u64>>,
}

impl Client{
	
	pub fn new(ID: &str, vectorSize: usize, port1: &str, port2: &str) -> Client{

    	let context = zmq::Context::new();
		let sender = context.socket(zmq::DEALER).unwrap();
		// let subscriber1 = context.socket(zmq::SUB).unwrap();

		let mut addr1: String = "tcp://localhost:".to_owned();
		let mut addr2: String = "tcp://localhost:".to_owned();

		addr1.push_str(port1);
		addr2.push_str(port2);
		sender.set_identity(ID.as_bytes());
		assert!(sender.connect(&addr1).is_ok());
		// assert!(subscriber1.connect(&addr2).is_ok());

		// let (tx, rx): (mpsc::Sender::<RecvType>, mpsc::Receiver::<RecvType>) = mpsc::channel();
		let ctx = context.clone();
		let buffer = Arc::new(RwLock::new(HashMap::<Vec<u8>, RecvType>::new()));
		let bf = buffer.clone();
    	let subThread = thread::spawn(move || {
    		let subscriber = ctx.socket(zmq::SUB).unwrap();
			assert!(subscriber.connect(&addr2).is_ok());
			subscriber.set_subscribe("".as_bytes());
			return sub_task(subscriber, bf)
	    });

	    let signKey = SigningKey::random(&mut OsRng);
	    let privateKey = EphemeralSecret::random(&mut OsRng);

		Client {

			context: context,
			sender: sender,
			//subscriber: subscriber1,

			// rx: rx,
			subThread: subThread,
			buffer: buffer,

			ID: ID.to_string(),

			veriKey: VerifyKey::from(&signKey),
			signKey: signKey,

			publicKey: EncodedPoint::from(&privateKey),
			privateKey: privateKey,

			clientVerikeys: Vec::<Vec<u8>>::new(),
			shareKeys: HashMap::new(),
			shareOrder: Vec::<Vec<u8>>::new(),				
			vectorSize: vectorSize,
			param: None,
			shares: Vec::new(),
		}
	}


	pub fn handshake(&mut self) -> Result<usize, ClientError> {

	/*
			Client say Helloo
			Server send unique signKey
			Generate veriKey from signKey
	*/

		match send(&self.sender, &format!("Hello, I'm {}", self.ID)) {
			Ok(_) => (),
			Err(_) => return Err(ClientError::SendFailure(0)),
		};
		let msg = recv(&self.sender);
		let sk = match msg {
			RecvType::bytes(b) => b,
			_ => return Err(ClientError::UnexpectedRecv(msg)),
		};
		match SigningKey::new(&sk) {
			Ok(k) => self.signKey = k,
			Err(_) => return Err(ClientError::EncryptionError(0)),
		};
		self.veriKey = VerifyKey::from(&self.signKey);

	/*
			Wait for Handshake finishing
			When state change
			Server send a list of veriKeys
	*/

		let waitRes = self.state_change_broadcast("HS");
		match waitRes {
			RecvType::matrix(m) => {
				println!("{} Recieved other's vk: {:?}", self.ID, &m.len());
				self.clientVerikeys = m
			},
			_ => return Err(ClientError::UnexpectedRecv(waitRes)),
		};
		println!("OK from handshake");
		return Ok(0)
	}


	pub fn key_exchange(&mut self) -> Result<usize, ClientError> {

	/*		 
			Generate Deffie-Helman key
			Sign DH key and send
	*/

		let publicKeyVec = self.publicKey.to_bytes();
		let signedPublicKey: Signature = self.signKey.sign(&publicKeyVec);

		let msg = vec![publicKeyVec.to_vec(),
					signedPublicKey.as_ref().to_vec()];
		match send_vecs(&self.sender, msg) {
			Ok(_) => (),
			Err(_) => return Err(ClientError::SendFailure(1)),
		};
		// Server says Ok
		let msg = recv(&self.sender);
		match msg {
			RecvType::string(s) => println!("{}, {:?}", self.ID, s),
			_ => return Err(ClientError::UnexpectedRecv(msg)),
		};

	/*		 
			Wait for state change
			Server recv all DH keys
			and sends everyone DH key list
			Create shared keys save as (DH pk, sharedKey)
	*/

		let waitRes = self.state_change_broadcast("KE");
		let publicKeys = match waitRes {
			RecvType::matrix(m) => m,
			_ => return Err(ClientError::UnexpectedRecv(waitRes)),
		};
		println!("{} Recieved other's pk: {:?}", self.ID,  &publicKeys.len());
		for pk in publicKeys.iter() {
			let shared = self.privateKey
							 .diffie_hellman(
							 	&EncodedPoint::from_bytes(pk).unwrap()
							 );
			match shared {
				Ok(s) => self.shareKeys.insert(pk.clone(), s.as_bytes().to_vec()),
				Err(_) => return Err(ClientError::EncryptionError(1)),
			};
		}
		self.shareOrder = publicKeys;
		println!("{}, OK from key_exchange", self.ID);
		return Ok(1) 
	}
	

	pub fn input_sharing(&mut self, input: &mut Vec<u64>) -> Result<usize, ClientError> {

	/*		 
			Wait for state change
			Recv sharing parameters
			Perform pss
	*/
		assert!(input.len() == self.vectorSize);
		let waitRes = self.state_change_broadcast("IS");
		let sharingParams = match waitRes {
			RecvType::bytes(m) => {
				assert_eq!(m.len(), 6*8);
				read_le_u64(m)
			},
			_ => return Err(ClientError::UnexpectedRecv(waitRes)),
		};

		let mut param = Param {
			P: sharingParams[0] as u128,
			R2: sharingParams[1] as u128,
			R3: sharingParams[2] as u128,
			D2: sharingParams[3] as usize,
			D3: sharingParams[4] as usize,
			L: sharingParams[5] as usize,			// in semi-honest, L = D2
			remainder: 0usize,
		};
		let N = self.shareKeys.len();
		let V = self.vectorSize;
		let L = param.L;
		let B = V/L;
		param.remainder = V - B * L;

		println!("{} param R2: {}, R3: {}, d2: {:?}, d3: {}, L {}", self.ID, param.R2, param.R3, param.D2, param.D3, L);		

		// V = B * L
		println!("B * L = {} * {}, N = {}",  B, L, N);

		let mut pss = PackedSecretSharing::new(
			param.P, param.R2, param.R3, 
			param.D2, param.D3, 5 * V, L, N
		);

	/*
		[xi1 xi2 ... xim ]
		[y1 y2 ... ym]
		[ai 0 ... 0 
		[a0i a1i ... 0]	//one bit of ai
		[r1 r2 ... rm]
	*/

		// Insert y = x^2
		let mut ySum = 0u128;
		for i in 0..V {
			let y = (input[i] as u128) * (input[i] as u128) % param.P;
			input.push(y as u64);
			ySum = (ySum + y) % param.P;
		}

		// Insert y sum
		input.push(ySum as u64);
		input.extend(vec![0; V-1]);
		
		// Insert bits of y sum
		let mut yBitArr = Vec::<u64>::new();
		while ySum > 0 {
			yBitArr.push((ySum % 2) as u64);
			ySum = ySum >> 1;
		}
		yBitArr.reverse();
		let yBitArr_len = yBitArr.len();
		assert!(yBitArr_len <= V);
		input.extend(yBitArr);
		for _ in yBitArr_len..V {
			input.push(0);
		}

		// Insert random
		for i in 0..V {
			input.push(OsRng.next_u64());
		}

		assert!(input.len() == 5 * V);
		let resultMatrix = pss.share(&input);

	/* 
		 	Encrypt shares for each DH sharedKey
			send [shares_c1, shares_c2, ....  ]
	*/
	
		println!("finished pss: (#Clients * sharesLen) = ({} * {}).. {}", 
			resultMatrix.len(), resultMatrix[0].len(), self.ID);
		let mut msg = Vec::new();
		for (i, pk) in self.shareOrder.iter().enumerate() {
			
			let shareKey = self.shareKeys.get(pk).unwrap();
			let k = GenericArray::from_slice(&shareKey);
			let cipher = Aes256Gcm::new(k);
			let nonce = GenericArray::from_slice(b"unique nonce");
			let mut shareBytes = Vec::new();
			for r in &resultMatrix[i] {
				shareBytes.extend(&r.to_le_bytes());
			}
			let encryptedShares = cipher.encrypt(nonce, shareBytes
										.as_slice())
		    					   		.expect("encryption failure!");	
		    msg.push(encryptedShares);
		}
		self.param = Some(param);
		match send_vecs(&self.sender, msg) {
			Ok(_) => return Ok(2),
			Err(_) => return Err(ClientError::SendFailure(2)),
		};
	}

	pub fn shares_collection(&mut self) -> Result<usize, ClientError> {
	/* 
			Loop to collect shares
			For each shares, Dec(sharedKey, msg)
			Then add to sum
	*/
		let param = self.param.unwrap();
		let N = self.shareKeys.len();
		let V = self.vectorSize;
		let L = param.L;
		let B = V/L;

		let mut cnt = 0;
		self.shares = vec![vec![0u64]; N];
		loop {
			let mut item = [self.sender.as_poll_item(zmq::POLLIN)];
	        zmq::poll(&mut item, 2).unwrap();
	        if item[0].is_readable() {
	        	let msg = recv(&self.sender);
	        	match msg {
	        	 	RecvType::matrix(m) => {
	        	 		assert!(m.len() == 2);
	        	 		let idx = self.shareOrder.iter().position(|s| s == &m[0]).unwrap();
	        	 		let cipher = match self.shareKeys.get(&m[0]) {
	        	 			Some(c) => {
								let k = GenericArray::from_slice(c);
								Aes256Gcm::new(k)
	        	 			},
	        	 			None => {
	        	 				println!("fail get client"); 
	        	 				return Err(ClientError::UnidentifiedShare(2));
	        	 			},
	        	 		};
						let nonce = GenericArray::from_slice(b"unique nonce");
			 			let plaintext = match cipher.decrypt(nonce, m[1].as_ref()) {
			 				Ok(p) => read_le_u64(p),
			 				Err(_) => {
			 					println!("fail decrypt"); 
			 					return Err(ClientError::EncryptionError(2));
			 				}
			 			};
			 			assert!(plaintext.len() == 5 * B);
			 			self.shares[idx] = plaintext;
			 			cnt += 1;
	        	 	},
	        	 	_ => return Err(ClientError::UnexpectedRecv(msg)),
	        	 };

	        };
	        // stop when recv shares from each peer
			if(cnt == N){
	        	break;
	        }
		}
		Ok(3)
	}

	pub fn error_correction(&mut self) -> Result<usize, ClientError> {
	/*
			Recv vecs for all tests
			[[dorpouts], [D test], [Input Bit test], ....]
			Handle dropouts
	*/
		let N = self.shareKeys.len();
		let V = self.vectorSize;
		let L = self.param.unwrap().L;
		let B = V/L;
		let P = self.param.unwrap().P;
		
		let waitRes = self.state_change_broadcast("EC");
		let mut dorpouts;
		let degTest;
		match waitRes {
			RecvType::matrix(m) => {
				if m.len() != 2 {							// [[dorpouts], [D test],....]
					return Err(ClientError::UnexpectedRecv(RecvType::matrix(m)));
				}
				dorpouts = read_le_u64(m[0].clone());
				degTest = read_le_u64(m[1].clone()); //TODO: Add more tests
				assert!(degTest.len() == 5 * B);
			},
			_ => return Err(ClientError::UnexpectedRecv(waitRes)),
		};
		println!("{:?} EC param (DT len {})", self.ID, degTest.len());
	/*
			Comput tests only for those who didn't dropout
			We don't remove anyone cuz resizing array is slow
	*/
		let mut msg = Vec::new();	//TODO: Add more tests	
		// D Test
		for i in 0..N {
			if dorpouts.contains(&(i as u64)) { 
				// assert shares recieved aligned with dropouts
				assert!(self.shares[i][0] == 0);
				continue; 
			}
			assert!(self.shares[i][0] != 0);
			let mut tests = vec![0u64; 7];
			// Degree Test
			for j in 0..5*B {
				tests[0] += ((degTest[j] as u128) * (self.shares[i][j] as u128) % P) as u64;
				tests[0] %= P as u64;
			}
			// TODO: more tests...
			println!("{:?} calculated 7 tests {:?} for client {}", self.ID,  tests, i);
			let mut tests_results = Vec::new();
			for t in tests {
				tests_results.extend((t as u64).to_le_bytes().to_vec());
			}
			msg.push(tests_results);
		}
		/* msg
			c0: [[t1, t2....t7]
			c1:  [t1, t2....t7]
			...
			cn:  [t1, t2....t7]]
		*/
		match send_vecs(&self.sender, msg) {
			Ok(_) => return Ok(4),
			Err(_) => return Err(ClientError::SendFailure(4)),
		};
	}

	pub fn aggregation(&self) -> Result<usize, ClientError> {
	/* 
		 	N*N shares
		 	Skip the rows of Ci who dropouts or fail
		 	Send [agggregation_bytes, signature]s
	*/
		let V = self.vectorSize;
		let L = self.param.unwrap().L;
		let B = V/L;
		let P = self.param.unwrap().P;

		let waitRes = self.state_change_broadcast("AG");
		let dropouts = match waitRes {
			RecvType::bytes(b) => read_le_u64(b),
			_ => return Err(ClientError::UnexpectedRecv(waitRes)),
		};
		println!("{:?} aggregation, skipping {:?}", self.ID, dropouts);
		let mut aggregation = vec![0u64; B];
		for i in 0..self.shares.len() {
			if !dropouts.contains(&(i as u64)) {
				for j in 0..B {
					aggregation[j] += ((aggregation[j] as u128 + self.shares[i][j] as u128) % P) as u64;
				}
			}
		}
		println!("{} sending aggregation {:?}", self.ID, aggregation.len());
		let mut aggregation_bytes = Vec::new();
		for a in aggregation.iter() {
			aggregation_bytes.extend(a.to_le_bytes().to_vec());
		}
		let msg = vec![
			aggregation_bytes.clone(),
			self.signKey.sign(&aggregation_bytes).as_ref().to_vec()
		];
		match send_vecs(&self.sender, msg) {
			Ok(_) => return Ok(3),
			Err(_) => return Err(ClientError::SendFailure(2)),
		};
		println!("OK from input_sharing");
	}

	pub fn state_change_broadcast(&self, curState: &str) -> RecvType {
	/*
		When state change
		loops till recieving information from subscriber buffer
	*/
		println!("{} waiting in {} ....", self.ID, curState);
		let curState = curState.as_bytes().as_ref();
		loop {
			match self.buffer.read() {
				Ok(guard) => {
					match guard.get(curState) {
						Some(m) => return m.clone(),
						None => sleep(Duration::from_millis(50)),
					}
				},
				Err(_) => return RecvType::string("".to_string()),
			};
		}
	}
}

fn sub_task(subscriber: zmq::Socket, 
	buffer: Arc<RwLock<HashMap<Vec<u8>, RecvType>>>) -> Result<usize, ClientError> {
    /*
		Subscriber thread
		Keep recieving from socket
		Consume msg emmited previously, add to buffer if it's new
    */
    loop {
        let (topic, data) = consume_broadcast(&subscriber);
        if buffer.read().unwrap().contains_key(&topic) {
            continue;
        }
        match buffer.write() {
            Ok(mut guard) => guard.insert(topic, data),
            Err(_) => return Err(ClientError::MutexLockFail(0)),
        };
    }
    Ok(0)
}

fn read_le_u128(input: Vec<u8>) -> Vec<u128> {
    let mut res = Vec::<u128>::new();
    if input.len() == 0 {
    	return res;
    }
    let mut ptr = &mut input.as_slice();
    loop {
        let (int_bytes, rest) = ptr.split_at(std::mem::size_of::<u128>());
        *ptr = rest;
        res.push(u128::from_le_bytes(int_bytes.try_into().unwrap()));
        if (rest.len() < 8) {
            break;
        }
    }
    res
}

fn read_le_u64(input: Vec<u8>) -> Vec<u64> {
    let mut res = Vec::<u64>::new();
    if input.len() == 0 {
    	return res;
    }
    let mut ptr = &mut input.as_slice();
    loop {
        let (int_bytes, rest) = ptr.split_at(std::mem::size_of::<u64>());
        *ptr = rest;
        res.push(u64::from_le_bytes(int_bytes.try_into().unwrap()));
        if (rest.len() < 8) {
            break;
        }
    }
    res
}



