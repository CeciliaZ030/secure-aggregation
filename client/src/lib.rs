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

		// Send hello to server
		match send(&self.sender, &format!("Hello, I'm {}", self.ID)) {
			Ok(_) => (),
			Err(_) => return Err(ClientError::SendFailure(0)),
		};

		// Server assign a unique private key
		// for signature
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
		// Wait for clients finish joining
		// State change
		// recieve others' verification key
		let waitRes = self.state_change_broadcast("HS");
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

		// Send Deffie-Helman key
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
			RecvType::string(s) => println!("{:?}", s),
			_ => return Err(ClientError::UnexpectedRecv(msg)),
		};

		// Wait for clients finish sending keys
		// State change
		// recieve others' Deffie-Hellman key	
		let waitRes = self.state_change_broadcast("KE");
		let publicKeys = match waitRes {
			RecvType::matrix(m) => m,
			_ => return Err(ClientError::UnexpectedRecv(waitRes)),
		};
		println!("Recieved other's pk: \n {:?}", &publicKeys);
		
		// Create shared keys for each clients
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
					self.shareKeys.insert(pk, s.as_bytes().to_vec());
				},
				Err(_) => return Err(ClientError::EncryptionError(1)),
			};
		}
		println!("OK from key_exchange");
		return Ok(1) 
	}
	

	pub fn input_sharing(&mut self, input: &Vec<u64>) -> Result<usize, ClientError> {

		let waitRes = self.state_change_broadcast("IS");
		let sharingParams = match waitRes {
			RecvType::bytes(m) => {
				assert_eq!(m.len(), 7*16);
				read_le_u128(m)
			},
			_ => return Err(ClientError::UnexpectedRecv(waitRes)),
		};

		let degreeTwo = sharingParams[0] as usize;
		let degreeThree = sharingParams[1]as usize;
		let blockLength = sharingParams[2] as usize;
		let numCorrupted = sharingParams[3];
		let prime = sharingParams[4];
		let rootTwo = sharingParams[5];
		let rootThree = sharingParams[6];
		let clientNum = self.shareKeys.len();
		let blockNum = input.len()/blockLength;

		println!("computing shares with param d2: {:?}, d3: {}, blocks: {} * {} per block, clientNum {}", degreeTwo, degreeThree, input.len()/blockLength, blockLength, clientNum);
		let mut pss = PackedSecretSharing::new(prime, 
											rootTwo, rootThree, 
											degreeTwo, degreeThree, 
											blockLength, clientNum);
		let mut resultMatrix = vec![vec![0u8; 0]; clientNum];

		for i in 0..input.len()/blockLength {
			let shares = pss.share_u64(
				&input[blockLength*i..blockLength+blockLength*i]	// on heap
			);
			println!("one poly {:?}", shares.len());
			for j in 0..clientNum {
				resultMatrix[j].extend((shares[j] as u64).to_le_bytes().to_vec())
			}
		}
		/* TODO: 
		 Make sure shares and keys are always in the same order
		*/
		println!("raw shares ({} * {})..", resultMatrix.len(), resultMatrix[0].len());
		for (i, (pk, shareKey)) in self.shareKeys.iter().enumerate() {
			
			// Encrypt with the share key
			let k = GenericArray::from_slice(&shareKey);
			let cipher = Aes256Gcm::new(k);
			let nonce = GenericArray::from_slice(b"unique nonce");
			let encryptedShares = cipher.encrypt(nonce, resultMatrix[i]
										.as_slice())
		    					   		.expect("encryption failure!");
			println!("encryptedShares len {:?}", encryptedShares.len());
			
			/* Prepend the pubKey of this peer 
		    so that server know who to forward
		    */
		    let mut msg = Vec::new();
		    msg.push(pk.clone());
		    msg.push(encryptedShares);

			match send_vecs(&self.sender, msg) {
				Ok(_) => continue,
				Err(_) => return Err(ClientError::SendFailure(1)),
			};
		}
		// Recv clients shares
		println!("recieving shares...");
		let mut aggregatedShares = vec![0u128; blockNum];
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
			 			println!("plaintext {:?}", plaintext);
			 			println!("aggregatedShares {:?}", aggregatedShares);

			 			for i in 0..plaintext.len() {
			 				aggregatedShares[i] = (aggregatedShares[i] + plaintext[i] as u128) % prime;
			 			}
			 			cnt += 1;
				        println!("{:?} has aggregated {} shares", self.ID, cnt);
				        if(cnt == clientNum){
				        	break;
				        }
	        	 	},
	        	 	RecvType::string(s) => println!("--{:?}", s),
	        	 	_ => return Err(ClientError::UnexpectedRecv(msg)),
	        	 };

	        };
		}

		self.state_change_broadcast("AG");

		let mut asBytes = Vec::new();
		println!("aggregatedShares {:?}", aggregatedShares);
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

		println!("waiting in {} ....", curState);
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

