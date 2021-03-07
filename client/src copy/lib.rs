use std::str;
use std::collections::HashMap;
use std::time::Duration;
use std::convert::TryInto;


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
	UnidentifiedShare(usize),
}


pub struct Client{

	context: zmq::Context,
	sender: zmq::Socket,
	subPort: String,

	pub ID: String,						//Unique ID that is field element

	signKey: SigningKey,				//Authentification
	veriKey: VerifyKey,

	privateKey: EphemeralSecret,		//ECDH
	publicKey: EncodedPoint,

	clientVerikeys: Vec<Vec<u8>>,
	shareKeys: HashMap<Vec<u8>, Vec<u8>>, // Array of (pubKey of peer, shareKey with peer)
}

impl Client{
	
	pub fn new(ID: &str, context: zmq::Context, port1: &str, port2: &str) -> Client{

		let sender = context.socket(zmq::DEALER).unwrap();

		let mut addr1: String = "tcp://localhost:".to_owned();
		let mut addr2: String = "tcp://localhost:".to_owned();

		addr1.push_str(port1);
		addr2.push_str(port2);
		sender.set_identity(ID.as_bytes());
		assert!(sender.connect(&addr1).is_ok());

		let signKey = SigningKey::random(&mut OsRng);
		let veriKey = VerifyKey::from(&signKey);
		let privateKey = EphemeralSecret::random(&mut OsRng);
		let publicKey = EncodedPoint::from(&privateKey);

		Client {

			context: context,
			sender: sender,
			subPort: addr2,

			ID: ID.to_string(),

			signKey: signKey,
			veriKey: veriKey,

			privateKey: privateKey,
			publicKey: publicKey,

			clientVerikeys: Vec::<Vec<u8>>::new(),
			shareKeys: HashMap::new(),
		}
	}


	pub fn handshake(&mut self) -> Result<usize, ClientError> {

	/*
			Client say Hello
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

		let waitRes = self.state_change_broadcast("IS");
		let sharingParams = match waitRes {
			RecvType::bytes(m) => {
				assert_eq!(m.len(), 5*16);
				read_le_u128(m)
			},
			_ => return Err(ClientError::UnexpectedRecv(waitRes)),
		};

		let degreeTwo = sharingParams[0] as usize;
		let degreeThree = sharingParams[1] as usize;
		let prime = sharingParams[2];
		let rootTwo = sharingParams[3];
		let rootThree = sharingParams[4];
		let N = self.shareKeys.len();
		let L = degreeTwo;
		let B = input.len()/L;

		println!("computing shares with param d2: {:?}, d3: {}, V = B * L = {} * {}, N {}", 
			degreeTwo, degreeThree, B, L, N);
		
		// V = B * L

		let mut pss = PackedSecretSharing::new(
			prime, rootTwo, rootThree, degreeTwo, degreeThree, L, N
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

		let remainderFg = B * L < input.len();
		if (remainderFg) {
			let rest = input.len() - B * L;
			
			println!("uneven L V = {} * {} + {:?} = {}", B, L, rest, input.len());
			
			let mut pss = PackedSecretSharing::new(
				prime, rootTwo, rootThree, degreeTwo, degreeThree, rest, N);
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

		println!("finished raw shares ({} * {})..", resultMatrix.len(), resultMatrix[0].len());
		
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
				Err(_) => return Err(ClientError::SendFailure(1)),
			};
		}

	/* 
		 	Aggregation stage
			Loop to collect shares
			For each shares, Dec(sharedKey, msg)
			Then add to sum
	*/

		println!("{} recieving shares back...", self.ID);
		let mut aggregatedShares = match remainderFg {
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
			 			//println!("plaintext {:?}", plaintext);
			 			for i in 0..plaintext.len() {
			 				aggregatedShares[i] = (aggregatedShares[i] + plaintext[i] as u128) % prime;
			 			}
			 			//println!("aggregatedShares {:?}", aggregatedShares);
			 			cnt += 1;
				        println!("{:?} has aggregated {} shares", self.ID, cnt);
	        	 	},
	        	 	RecvType::string(s) => println!("--{:?}", s),
	        	 	_ => return Err(ClientError::UnexpectedRecv(msg)),
	        	 };

	        };
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

		let subscriber = self.context.socket(zmq::SUB).unwrap();
		assert!(subscriber.connect(&self.subPort).is_ok());

		println!("{} waiting in {} ....", self.ID, curState);
		subscriber.set_subscribe(curState.as_bytes()).unwrap();
		
		loop {
			let mut item = [subscriber.as_poll_item(zmq::POLLIN)];
	        zmq::poll(&mut item, 2).unwrap();
	        if item[0].is_readable() {
	        	println!("state change {}", curState);
	        	return recv_broadcast(&subscriber)
	        }
		}
	}

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

