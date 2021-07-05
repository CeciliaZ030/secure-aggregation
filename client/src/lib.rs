#![allow(unused_imports)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]
#![allow(unused_must_use)]

use std::str;
use std::collections::HashMap;
use std::time::{Duration, Instant};
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
mod util;
use sockets::*;
use util::*;

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
}


pub struct Client{

	pub ID: String,							//Unique ID that is field element
	context: zmq::Context,
	sender: zmq::Socket,

	subRx: mpsc::Receiver<Vec<u64>>,
	subThread :thread::JoinHandle<Result<usize, ClientError>>,
	buffer: Arc<RwLock<HashMap<Vec<u8>, RecvType>>>,

	signKey: SigningKey,					//Authentification
	veriKey: VerifyKey,

	privateKey: EphemeralSecret,			//ECDH
	publicKey: EncodedPoint,

	clientVerikeys: Vec<Vec<u8>>,
	shareKeys: HashMap<Vec<u8>, Vec<u8>>, 	// {key: DH pubKey, value: DH shareKey}
	shareOrder:  Vec<Vec<u8>>,				/* [pk_c1, pk_c2, ....] 
											all clients assign shares in this order
											*/
	vectorSize: usize,
	inputBitLimit: Option<usize>,
	param: Option<Param>,
	shares: Vec<Vec<u64>>,
}


impl Client{
	
	pub fn new(ID: &str, vectorSize: usize, inputBitLimit: Option<usize>,
		ip: Option<&str>, port1: usize, port2: usize) -> Client{

    	let context = zmq::Context::new();
		let sender = context.socket(zmq::DEALER).unwrap();

		let mut addr1: String;
		let mut addr2: String;

		match ip {
			Some(address) => {
				addr1 = format!("tcp://{}:{:?}", address, port1);
				addr2 = format!("tcp://{}:{:?}", address, port2);
				println!("Sender connecting {}", addr1);
				println!("Subscriber connecting to {}", addr2);			
			},
			None => {
				addr1 = format!("tcp://localhost:{:?}", port1);
				addr2 = format!("tcp://localhost:{:?}", port2);
				println!("Sender going default {}", addr1);
				println!("Subscriber going default {}", addr2);
			},
		}

		sender.set_identity(ID.as_bytes());
		assert!(sender.connect(&addr1).is_ok());

		let ctx = context.clone();
		let buffer = Arc::new(RwLock::new(HashMap::<Vec<u8>, RecvType>::new()));
		let bf = buffer.clone();
    	let (tx, rx) = mpsc::channel();
    	let subThread = thread::spawn(move || {
    		let subscriber = ctx.socket(zmq::SUB).unwrap();
			assert!(subscriber.connect(&addr2).is_ok());
			subscriber.set_subscribe("".as_bytes());
			return sub_task(subscriber, bf, tx)
	    });

	    let signKey = SigningKey::random(&mut OsRng);
	    let privateKey = EphemeralSecret::random(&mut OsRng);

		Client {

			context: context,
			sender: sender,

			subRx: rx,
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
			inputBitLimit: inputBitLimit,
			param: None,
			shares: Vec::new(),
		}
	}


	pub fn handshake(&mut self) -> Result<usize, ClientError> {
		let BENCH_TIMER = Instant::now();
	/*
			Client say Hello
			Server send unique signKey
			Generate veriKey from signKey
	*/
		match send(&self.sender, &format!("Hello, I'm {}", self.ID)) {
			Ok(_) => (),
			Err(_) => return Err(ClientError::SendFailure(1)),
		};
		let msg = recv(&self.sender);
		let sk = match msg {
			RecvType::bytes(b) => b,
			_ => return Err(ClientError::UnexpectedRecv(msg)),
		};
		match SigningKey::new(&sk) {
			Ok(k) => self.signKey = k,
			Err(_) => return Err(ClientError::EncryptionError(1)),
		};
		self.veriKey = VerifyKey::from(&self.signKey);

	/*
			Wait for Handshake finishing
			When state change
			Server send a list of veriKeys
	*/
		let BEFORE = Instant::now();
		let waitRes = self.state_change_broadcast("HS");
		let AFTER = Instant::now();
		match waitRes {
			RecvType::matrix(m) => {
				self.clientVerikeys = m
			},
			_ => return Err(ClientError::UnexpectedRecv(waitRes)),
		};
		println!("State 1 elapse {:?}ms ({})", 
			(BEFORE-BENCH_TIMER+AFTER.elapsed()).as_millis(), self.ID);
		return Ok(1)
	}


	pub fn key_exchange(&mut self) -> Result<usize, ClientError> {
		let BENCH_TIMER = Instant::now();
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
			Err(_) => return Err(ClientError::SendFailure(2)),
		};
		// Server says Ok
		let msg = recv(&self.sender);
		match msg {
			RecvType::string(s) => (),
			_ => return Err(ClientError::UnexpectedRecv(msg)),
		};
	/*		 
			Wait for state change
			Server recv all DH keys
			and sends everyone DH key list
			Create shared keys save as (DH pk, sharedKey)
	*/
		let BEFORE = Instant::now();
		let waitRes = self.state_change_broadcast("KE");
		let publicKeys = match waitRes {
			RecvType::matrix(m) => m,
			_ => return Err(ClientError::UnexpectedRecv(waitRes)),
		};
		let AFTER = Instant::now();
		for pk in publicKeys.iter() {
			let shared = self.privateKey
							 .diffie_hellman(
							 	&EncodedPoint::from_bytes(pk).unwrap()
							 );
			match shared {
				Ok(s) => self.shareKeys.insert(pk.clone(), s.as_bytes().to_vec()),
				Err(_) => return Err(ClientError::EncryptionError(2)),
			};
		}
		self.shareOrder = publicKeys;
		println!("State 2 elapse {:?}ms ({})", 
			(BEFORE-BENCH_TIMER+AFTER.elapsed()).as_millis(), self.ID);
		return Ok(2) 
	}
	

	pub fn input_sharing_ml(&mut self, input: &mut Vec<u64>) -> Result<usize, ClientError> {
	/*		 
			Wait for state change
			Recv sharing parameters
			Calculate EC matrix: [x, y, randomness...]
			Perform pss
	*/
		assert!(input.len() == self.vectorSize);

		let waitRes = self.state_change_broadcast("IS");
		let BENCH_TIMER = Instant::now();

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
		};
		
		let N = self.shareKeys.len();
		let V = self.vectorSize;
		let L = param.L;
		let B = V/L;
		let S = self.inputBitLimit.unwrap();
		let P = param.P as u64;
		let Y = (
			((2f32*(S as f32) + (V as f32).log2().ceil())/
			(L as f32)).ceil()*
			(L as f32)) as usize;

		let x = input.clone();
	/*
			NOTE: heavy communication overhead if limit bit number
			if L is small and bitnum of ysum <= L
			the input bit limit will be very small!
			(u64 to share only ~5 bits input ???)

			[x1 x2 ... xv]
			============== v
			[y1 y2 ... yv]
			[ysum 02 ... 0l]
			[ysum_b1 ysum_b2 ... ]
			[ysum_bn 0n+1 ... 0l]
			============== 2v+2l
			[x1_b1 ... xl_b1]    	maxinum length of ysum 2S+log(V) celling [0, 2, 4]
			[x1_b2 ... xl_b2]
			...
			[x1_bs ... xl_bs]
			_______________
			[xl+1_b1 ... x2l_b1]
			[xl+1_b2 ... x2l_b2]
			...
			[xl+1_bs ... x2l_bs]
			_______________
			...
			_______________
			[x(b-1)l+1_b1 ... xbl_b1]
			[x(b-1)l+1_b2 ... xbl_b2]
			...
			[x(b-1)l+1_bs ... xbl_bs]
			============== 2v+2l+bsl
			[rC1 rC2 ... rCl]
			[rA1 rA2 ... rAl]
			[rB1 rB2 ... rBl]
			============== 2v+2l+bsl+3l
	*/
		// Insert y = x^2
		let mut ySum = 0;
		for i in 0..V {
			let y = ((x[i] as u128) * (x[i] as u128) % param.P) as u64;
			input.push(y);
			ySum = (ySum + y) % P;
		}
		//println!("ySum {:?}, {}", ySum, (ySum as f64).log(2.0));

		// Insert ysum
		input.push(ySum);
		input.extend(vec![0; L-1]);
		

		// Insert bits of ysum
		//	bitnum of ysum <= L
		let mut yBitArr = into_be_u64_vec(ySum as u64, Y);
		assert!(yBitArr.len() == Y);
		input.extend(yBitArr.clone());

		// Insert bits of x
		//	bitnum of xi <= S
		let mut x_bits = vec![0; L*S*B];
		for i in 0..V {
			let b = i/L; // b-th block
			let xi_bits = into_be_u64_vec(x[i].clone(), S);
			assert!(xi_bits.len() == S);
			for j in 0..S {
				x_bits[((b*S+j)*L)+(i-L*b)] = xi_bits[j];
			}
		}
		input.extend(x_bits);

		// Insert random C
		for i in 0..L {
			input.push(OsRng.next_u64() % P);
		}

		// Insert random A
		input.extend(vec![0u64; L]);

		// Insert random B
		let mut rand_sum = 0;
		for i in 0..L-1 {
			let rand = OsRng.next_u64() % P;
			input.push(rand);
			rand_sum = (rand_sum + rand) % P;
		}
		input.push(P - rand_sum);

		assert!(input.len() == 2*V + L + Y + L*S*B + 3*L);
		let mut pss = PackedSecretSharing::new(
			param.P, param.R2, param.R3, 
			param.D2, param.D3,
			2*V+L+Y+L*S*B+3*L, 
			L, N
		);
		let SHARE_START = Instant::now();
		let resultMatrix = pss.share(&input);
		assert!(resultMatrix.len() == N);
		assert!(resultMatrix[0].len() == (2*V + L + Y + L*S*B + 3*L)/L);
		println!("{:?} sharing time {:?}", self.ID, SHARE_START.elapsed().as_millis());
	/* 
		 	Encrypt shares for each DH sharedKey
			send [shares_c1, shares_c2, ....  ]
	*/
		let mut msg = Vec::new();
		for (i, pk) in self.shareOrder.iter().enumerate() {
			
			let shareKey = self.shareKeys.get(pk).unwrap();
			let k = GenericArray::from_slice(&shareKey);
			let cipher = Aes256Gcm::new(k);
			let nonce = GenericArray::from_slice(b"unique nonce"); //Self.pk
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
		println!("State 3 elapse {:?}ms ({})", BENCH_TIMER.elapsed().as_millis(), self.ID);
		match send_vecs(&self.sender, msg) {
			Ok(_) => {
				return Ok(3)
			},
			Err(_) => return Err(ClientError::SendFailure(3)),
		};
	}

pub fn input_sharing_sh(&mut self, input: &mut Vec<u64>) -> Result<usize, ClientError> {
	/*		 
			Wait for state change
			Recv sharing parameters
			Perform pss
	*/
		assert!(input.len() == self.vectorSize);

		let waitRes = self.state_change_broadcast("IS");
		let BENCH_TIMER = Instant::now();

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
			L: sharingParams[5] as usize,
		};
		
		let N = self.shareKeys.len();
		let V = self.vectorSize;
		let L = param.L;
		let B = V/L;
		let P = param.P as u64;

		assert!(input.len() == V);
		let mut pss = PackedSecretSharing::new(
			param.P, param.R2, param.R3, 
			param.D2, param.D3,
			V, L, N
		);
		let SHARE_START = Instant::now();
		let resultMatrix = pss.share(&input);
		assert!(resultMatrix.len() == N);
		assert!(resultMatrix[0].len() == B);
		println!("{:?} sharing time {:?}", self.ID, SHARE_START.elapsed().as_millis());
	/* 
		 	Encrypt shares for each DH sharedKey
			send [shares_c1, shares_c2, ....  ]
	*/
		let mut msg = Vec::new();
		for (i, pk) in self.shareOrder.iter().enumerate() {
			
			let shareKey = self.shareKeys.get(pk).unwrap();
			let k = GenericArray::from_slice(&shareKey);
			let cipher = Aes256Gcm::new(k);
			let nonce = GenericArray::from_slice(b"unique nonce"); //Self.pk
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
		println!("State 3 elapse {:?}ms ({})", BENCH_TIMER.elapsed().as_millis(), self.ID);
		match send_vecs(&self.sender, msg) {
			Ok(_) => {
				return Ok(3)
			},
			Err(_) => return Err(ClientError::SendFailure(3)),
		};
	}

	pub fn shares_recieving(&mut self) -> Result<usize, ClientError> {
		let BENCH_TIMER = Instant::now();
	/* 
			Loop to collect shares
			For each shares, Dec(sharedKey, msg)
			Then add to sum
	*/
		let N = self.shareKeys.len();
		let mut cnt = 0;
		self.shares = vec![vec![0u64]; N];
		loop {
			match self.subRx.try_recv() {
				Ok(dropouts) => {
					/* server broadcast dropouts
					break from waiting for shares...
					*/
					if dropouts.len() == 0 {continue;}
					for d in dropouts {
						assert!(self.shares[d as usize] == vec![0u64]);
					}
					break;
				},
				Err(_) => (),
			};
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
	        	 				return Err(ClientError::UnidentifiedShare(4));
	        	 			},
	        	 		};
						let nonce = GenericArray::from_slice(b"unique nonce");
			 			let plaintext = match cipher.decrypt(nonce, m[1].as_ref()) {
			 				Ok(p) => read_le_u64(p),
			 				Err(_) => {
			 					println!("fail decrypt"); 
			 					return Err(ClientError::EncryptionError(4));
			 				}
			 			};
			 			//assert!(plaintext.len() == (2*V + L + Y + L*S*B + 3*L)/L);
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
		println!("State 4 elapse {:?}ms ({})", BENCH_TIMER.elapsed().as_millis(), self.ID);
		Ok(3)
	}

	pub fn error_correction(&mut self) -> Result<usize, ClientError> {
	/*
			Recv vecs for all tests
			[[dorpouts], [Degree Test], [Input Bit Test], [Quadratic Test], 
			 [Input bound test], [L2-norm sum test], [L2-norm bit test], [L2-norm bound test]]
			Handle dropouts
	*/
		let N = self.shareKeys.len();
		let V = self.vectorSize;
		let L = self.param.unwrap().L;
		let B = V/L;
		let P = self.param.unwrap().P;
		let S = self.inputBitLimit.unwrap();
		let Y = (
			((2f32*(S as f32) + (V as f32).log2().ceil())/
			(L as f32)).ceil()*
			(L as f32)) as usize;

		let idx = self.shareOrder.iter().position(|s| s == &*self.publicKey.to_bytes()).unwrap();
		let waitRes = self.state_change_broadcast("EC");
		let BENCH_TIMER = Instant::now();

		let mut dorpouts;
		let (mut degree_rand, mut input_bit_rand, mut quadratic_rand, mut input_bound_rand, 
			mut l2_norm_bit_rand, mut l2_norm_sum_rand, mut l2_norm_bound_rand, mut l2_norm_bound_shares);
		match waitRes {
			RecvType::matrix(m) => {
				if m.len() != 9 {
					return Err(ClientError::UnexpectedRecv(RecvType::matrix(m)));
				}
				dorpouts = read_le_u64(m[0].clone());
				degree_rand = read_le_u64(m[1].clone());
				input_bit_rand = read_le_u64(m[2].clone());
				quadratic_rand = read_le_u64(m[3].clone());
				input_bound_rand = read_le_u64(m[4].clone());
				l2_norm_sum_rand = read_le_u64(m[5].clone());
				l2_norm_bit_rand = read_le_u64(m[6].clone());
				l2_norm_bound_rand = read_le_u64(m[7].clone());
				l2_norm_bound_shares = read_le_u64(m[8].clone());
			},
			_ => return Err(ClientError::UnexpectedRecv(waitRes)),
		};
	/*
			Comput tests only for those who didn't dropout
			Leave tests_bytes empty for row_i if client_i dropouts
			msg = 
				c0: [[t1, t2....t3]
				c1:  [t1, t2....t3]
				...
				ck:	 [] 				<- dropout
				...
				cn:  [t1, t2....t3]]
			We don't remove anyone cuz resizing array is slow
	*/
		let mut msg = Vec::new();
		for i in 0..N {
			let mut tests = Vec::new();
			if !dorpouts.contains(&(i as u64)) { 
				assert!(self.shares[i] != vec![0u64]);
				tests = vec![0u64; 3];

				// Degree Test
				let mut DT = 0u128;
				for j in 0..(2*V + L + Y + L*S*B + 3*L)/L {
					// r * each share
					DT += ((degree_rand[j] as u128) * (self.shares[i][j] as u128) % P);
					DT %= P;
				}
				tests[0] = (DT as u64).try_into().unwrap();
		// _________________________________________________________

				// Input Bit Test
				let mut IBTT = 0u128;
				for j in 0..B*S {
					// r * (x_bit * (1 - x_bit))
					let x_bit = self.shares[i][(2*V + L + Y)/L + j] as u128;
					IBTT += (x_bit * (1 + P - x_bit) % P) * (input_bit_rand[j] as u128) % P;
					IBTT %= P;
				}

				// Quadratic Test
				let mut QT = 0u128;
				for j in 0..B {
					// r * (x^2 - y)
					let x = self.shares[i][j] as u128;
					let y = self.shares[i][V/L + j] as u128;
					QT += ((x * x) % P + P - y) * (quadratic_rand[j] as u128) % P;
					QT %= P;
				}

				// L2-norm bit test
				let mut L2NBTT = 0u128;
				// r * (ySum_bits * (1 - ySum_bits))
				for j in 0..Y/L {
					let ySum_bits = self.shares[i][(2*V + L)/L + j] as u128;
					L2NBTT += (ySum_bits * (1 + P - ySum_bits) % P) * (l2_norm_bit_rand[j] as u128) % P;
					L2NBTT %= P;
				}

			/* 
			sum of three tests + randomness A generated by Party i (all 0)
			*/
				let sumA = ((IBTT + QT + L2NBTT) % P + self.shares[i][(2*V + L + Y + L*S*B + 1*L)/L] as u128) % P;
				tests[1] = (sumA as u64).try_into().unwrap();

		// _________________________________________________________

				// Input Bound Test
				let mut IBDT = 0u128;
				for j in 0..B {
					// r * ( sum(x_bit * 2^k) - x)
					let x = self.shares[i][j] as u128;
					let mut sumX_bit = 0u128;
					for k in 0..S {
						let x_bit = self.shares[i][(2*V + L + Y + j*S*L)/L + k] as u128;
						sumX_bit += x_bit * 2u128.pow(k as u32) % P;
						sumX_bit %= P;
					}
					IBDT += (sumX_bit + P - x) * (input_bound_rand[j] as u128) % P;
					IBDT %= P;
				}

				// L2-norm sum test
				let mut L2NST;
				let mut sumY = 0u128;
				for j in 0..B {
					let y = self.shares[i][V/L + j] as u128;
					sumY += y % P;
				}
				// r * (sum(y) - ySum)
				L2NST = (sumY + P - self.shares[i][(2*V)/L] as u128) * (l2_norm_sum_rand[0] as u128) % P;
				
				// L2-norm bound test
				let mut L2NBDT;
				let ySum = self.shares[i][(2*V)/L] as u128;
				// r * (ySum_bits * 2^k_share - ySum) [0,2,4,8,16,....]
				let mut share_sum = 0u128;
				for j in 0..Y/L {
					let ySum_bits = self.shares[i][(2*V + L)/L + j] as u128;
					share_sum += (ySum_bits * l2_norm_bound_shares[idx*(Y/L)+j] as u128) % P;
					share_sum %= P;
				}
				L2NBDT = (ySum + P - share_sum) % P * (l2_norm_bound_rand[0] as u128) % P;
				
			/* 
			sum of three tests + canceling randomness B generated by Party i (sum to 0)
			*/
				let sumB = ((IBDT + L2NST + L2NBDT) % P + self.shares[i][(2*V + L + Y + L*S*B + 2*L)/L] as u128) % P;
				tests[2] = (sumB as u64).try_into().unwrap();
			}
			msg.push(write_u64_le_u8(tests.as_slice()).to_vec());
		}
		println!("State 5 elapse {:?}ms ({})", BENCH_TIMER.elapsed().as_millis(), self.ID);
		match send_vecs(&self.sender, msg) {
			Ok(_) => {
				return Ok(5)
			},
			Err(_) => return Err(ClientError::SendFailure(5)),
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
		let BENCH_TIMER = Instant::now();
		let dropouts = match waitRes {
			RecvType::bytes(b) => read_le_u64(b),
			_ => return Err(ClientError::UnexpectedRecv(waitRes)),
		};
		println!("{:?} aggregation, skipping {:?}", self.ID, dropouts);
		let mut aggregation = vec![0u64; B];
		for i in 0..self.shares.len() {
			if !dropouts.contains(&(i as u64)) {
				for j in 0..B {
					aggregation[j] = ((aggregation[j] as u128 + self.shares[i][j] as u128) % P) as u64;
				}
			}
		}
		//println!("{} sending aggregation[0] {:?}, len {}", self.ID, aggregation[0], aggregation.len());
		let mut aggregation_bytes = Vec::new();
		for a in aggregation.iter() {
			aggregation_bytes.extend(a.to_le_bytes().to_vec());
		}
		let msg = vec![
			aggregation_bytes.clone(),
			self.signKey.sign(&aggregation_bytes).as_ref().to_vec()
		];
		println!("State 6 elapse {:?}ms ({})", BENCH_TIMER.elapsed().as_millis(), self.ID);
		match send_vecs(&self.sender, msg.clone()) {
			Ok(_) => (),//println!("{:?} sent input_sharing {:?}", self.ID, msg[0][0]),
			Err(_) => return Err(ClientError::SendFailure(5)),
		};
		return Ok(3);
	}

	pub fn state_change_broadcast(&self, curState: &str) -> RecvType {
	/*
		When state change
		loops till recieving information from subscriber buffer
	*/
		//println!("{} waiting in {} ....", self.ID, curState);
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
