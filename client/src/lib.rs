use std::str;
use std::collections::HashMap;
use std::time::Duration;
use std::convert::TryInto;
use std::sync::*;
use std::thread;
use std::thread::sleep;

use zmq::SNDMORE;
use zmq::Message;

use rand_core::OsRng;
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

use packed_secret_sharing::packed::*;

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
	degree2: usize,
	degree3: usize,
	prime: u128,
	root2: u128,
	root3: u128,
	remainderFg: bool,
}


pub struct Client{

	pub ID: String,						//Unique ID that is field element
	context: zmq::Context,
	sender: zmq::Socket,

	subThread :thread::JoinHandle<Result<usize, ClientError>>,
	buffer: Arc<RwLock<HashMap<Vec<u8>, RecvType>>>,

	signKey: SigningKey,				//Authentification
	veriKey: VerifyKey,

	privateKey: EphemeralSecret,		//ECDH
	publicKey: EncodedPoint,

	clientVerikeys: Vec<Vec<u8>>,
	shareKeys: HashMap<Vec<u8>, Vec<u8>>, // Array of (pubKey of peer, shareKey with peer)

	vectorSize: usize,
	param: Option<Param>,
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

			vectorSize: vectorSize,
			param: None,
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
				self.clientVerikeys = m;
			},
			_ => return Err(ClientError::UnexpectedRecv(waitRes)),
		}
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
		for pk in publicKeys {
			let shared = self.privateKey
							 .diffie_hellman(
							 	&EncodedPoint::from_bytes(&pk).unwrap()
							 );
			match shared {
				Ok(s) => {
					self.shareKeys.insert(pk, s.as_bytes().to_vec());
				},
				Err(_) => return Err(ClientError::EncryptionError(1)),
			};
		}
		println!("OK from key_exchange");
		return Ok(1) 
	}
	


	pub fn input_sharing(&mut self, input: &Vec<u64>) -> Result<usize, ClientError> {

	/*		 
			Wait for state change
			Recv sharing parameters
			Perform pss
	*/
		assert!(input.len() == self.vectorSize);
		let waitRes = self.state_change_broadcast("IS");
		let sharingParams = match waitRes {
			RecvType::bytes(m) => {
				assert_eq!(m.len(), 5*16);
				read_le_u128(m)
			},
			_ => return Err(ClientError::UnexpectedRecv(waitRes)),
		};

		let mut param = Param {
			degree2: sharingParams[0] as usize,
			degree3: sharingParams[1] as usize,
			prime: sharingParams[2],
			root2: sharingParams[3],
			root3: sharingParams[4],
			remainderFg: false,
		};
		self.param = Some(param);
		let N = self.shareKeys.len();
		let L = param.degree2;
		let B = input.len()/L;

		println!("computing shares with param d2: {:?}, d3: {}, V = B * L = {} * {}, N {}", 
			param.degree2, param.degree3, B, L, N);
		
		// V = B * L

		let mut pss = PackedSecretSharing::new(
			param.prime, param.root2, param.root3, param.degree2, param.degree3, L, N
		);
		let mut resultMatrix = vec![vec![0u8; 0]; N];
		for i in 0..B {
			let shares = pss.share_u64(
				&input[L*i..L+L*i]);
			for j in 0..N {
				resultMatrix[j].extend((shares[j] as u64).to_le_bytes().to_vec())
			}
		}

		// V = B * L + remains

		param.remainderFg = B * L < input.len();
		if (param.remainderFg) {
			let shares = pss.share_u64(&input[B * L..input.len()]);
			for j in 0..N {
				resultMatrix[j].extend((shares[j] as u64).to_le_bytes().to_vec())
			}
		}

	/* 
		 	Encrypt shares for each DH sharedKey
			Send with format: 
			(pk, Enc(sharedKey, shares))
	*/

		println!("finished pss: (#Clients * sharesLen) = ({} * {})..", resultMatrix.len(), resultMatrix[0].len());
		
		for (i, (pk, shareKey)) in self.shareKeys.iter().enumerate() {
			
			let k = GenericArray::from_slice(&shareKey);
			let cipher = Aes256Gcm::new(k);
			let nonce = GenericArray::from_slice(b"unique nonce");
			let encryptedShares = cipher.encrypt(nonce, resultMatrix[i]
										.as_slice())
		    					   		.expect("encryption failure!");	
		    let mut msg = Vec::new();
		    msg.push(pk.clone());
		    msg.push(encryptedShares);
			match send_vecs(&self.sender, msg) {
				Ok(_) => continue,
				Err(_) => return Err(ClientError::SendFailure(2)),
			};
		}
		Ok(2)
	}


	pub fn aggregation(&mut self) -> Result<usize, ClientError> {
	/* 
		 	Aggregation stage
			Loop to collect shares
			For each shares, Dec(sharedKey, msg)
			Then add to sum
	*/
		let param = self.param.unwrap();
		let N = self.shareKeys.len();
		let L = param.degree2;
		let B = self.vectorSize/L;

		let mut aggregatedShares = match param.remainderFg {
			false => vec![0u128; B],						// V = B * L
			true => vec![0u128; B + 1],						// V = B * L + remains
		};
		let mut cnt = 0;
		loop {
			let mut item = [self.sender.as_poll_item(zmq::POLLIN)];
	        zmq::poll(&mut item, 2).unwrap();
	        if item[0].is_readable() {
	        	let msg = recv(&self.sender);
	        	match msg {
	        	 	RecvType::matrix(m) => {
	        	 		assert!(m.len() == 2);
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
			 			for i in 0..plaintext.len() {
			 				aggregatedShares[i] = (aggregatedShares[i] + plaintext[i] as u128) % param.prime;
			 			}
			 			cnt += 1;
				        println!("{:?} has aggregated {} shares", self.ID, cnt);
	        	 	},
	        	 	// Reciep from server
	        	 	// 	"Input from client a to client b is successfull"
	        	 	RecvType::string(s) => println!("--{:?}", s),
	        	 	_ => return Err(ClientError::UnexpectedRecv(msg)),
	        	 };

	        };
	        // stop when recv shares from each peer
			if(cnt == N){
	        	break;
	        }
		}

	/* 
		 	Server has forwarded N*N shares
		 	Tells clients to aggregate
		 	Send aggregated vector
	*/
		self.state_change_broadcast("AG");
		let mut asBytes = Vec::new();
		println!("{} sending aggregatedShares {:?}", self.ID, aggregatedShares.len());
		for a in aggregatedShares {
			asBytes.extend((a as u64).to_le_bytes().to_vec());
		}
		let msg = vec![
			asBytes.clone(),
			self.signKey.sign(&asBytes).as_ref().to_vec()
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
        let (topic, data) = recv_broadcast1(&subscriber);
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



