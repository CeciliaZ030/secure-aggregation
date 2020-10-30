use zmq::SNDMORE;
use zmq::Message;
use std::str;

use rand_core::{OsRng};
use p256::{
	EncodedPoint,
	ecdh::{EphemeralSecret, SharedSecret},
    ecdsa::{SigningKey, Signature, signature::Signer, VerifyKey},
};
use aes_gcm::Aes256Gcm; // Or `Aes128Gcm`
use aes_gcm::aead::generic_array::GenericArray;

use packed_secret_sharing::*
use packed_secret_sharing::packed::*;

pub struct Client{

	pub sender: zmq::Socket,
	pub subscriber: zmq::Socket,

	pub ID: String,						//Unique ID that is field element

	signKey: SigningKey,				//Authentification
	veriKey: VerifyKey,

	privateKey: EphemeralSecret,		//ECDH
	publicKey: EncodedPoint,

	client_list: Vec<String>,
	client_verikeys: Vec<Vec<u8>>,
	shareKeys: Vec<SharedSecret>,
}

impl Client{
	
	pub fn new(ID: &str, context: zmq::Context, port1: &str, port2: &str) -> Client{

		let sender = context.socket(zmq::DEALER).unwrap();
		let subscriber = context.socket(zmq::SUB).unwrap();

		let mut addr1: String = "tcp://localhost:".to_owned();
		let mut addr2: String = "tcp://localhost:".to_owned();

		addr1.push_str(port1);
		addr2.push_str(port2);
		sender.set_identity(ID.as_bytes());
		assert!(sender.connect(&addr1).is_ok());
		assert!(subscriber.connect(&addr2).is_ok());

		let signKey = SigningKey::random(&mut OsRng);
		let veriKey = VerifyKey::from(&signKey);
		let privateKey = EphemeralSecret::random(&mut OsRng);
		let publicKey = EncodedPoint::from(&privateKey);

		Client {

			sender: sender,
			subscriber: subscriber,

			ID: ID.to_string(),

			signKey: signKey,
			veriKey: veriKey,

			privateKey: privateKey,
			publicKey: publicKey,

			client_list: Vec::<String>::new(),
			client_verikeys: Vec::<Vec<u8>>::new(),
			shareKeys: Vec::<SharedSecret>::new(),
		}
	}

	
	pub fn recv_strings(&self) -> Vec<String> {
		let mut res : Vec<String> = Vec::new();
		let msg = self.sender.recv_multipart(0).unwrap();
        for m in msg {
            res.push(String::from_utf8(m).unwrap());
        }
        println!("{:?}", res);
        res
	}

	pub fn recv_vecs(&self) -> Vec<Vec<u8>> {
		self.sender.recv_multipart(0).unwrap()
	}
	/*
	//pub fn recv_multipart(&self, flags: i32) -> Result<Vec<Vec<u8>>>
    //pub fn recv_string(&self, flags: i32) -> Result<result::Result<String, Vec<u8>>> 
    //pub fn recv_bytes(&self, flags: i32) -> Result<Vec<u8>> 
	//pub fn recv_into(&self, bytes: &mut [u8], flags: i32) -> Result<usize>
	*/
	pub fn recv_broadcast(&self, topic: &str) -> Vec<Vec<u8>> {
		self.subscriber.set_subscribe(topic.as_bytes());
		let mut message;
		loop {
			match self.subscriber.recv_multipart(0) {
				Ok(msg) => {
						message = msg;
						break;
					},
				Err(_) => panic!("Failed to recieve braoadcast."),
			}
		}
		message.remove(0);
		message
	}

    // Data can be Vec<Vec<u8>> or Vec<String> or Vec<str>
    pub fn send_vec<I, T>(&self, data: I) -> Result<&str, &str>
    where 
        I: IntoIterator<Item = T>,
        T: Into<Message>,
    {

        let result = self.sender.send_multipart(data, 0);
        match result {
            Ok(T) => Ok("Sent vector successfully."),
            Err(Error) => Err("Failed sending vector."),
        }
    }

    // Data can be Vec<u8> or &str
    pub fn send<T>(&self, data: T) -> Result<&str, &str>
    where
        T: Into<Message>,
    {
        let result = self.sender.send(data, 0);
        match result {
            Ok(T) => Ok("Sent message successfully."),
            Err(Error) => Err("Failed sending message."),
        }
    }

	pub fn handshake(&mut self) -> Result<u32, &str> {
		
		let hello = "Hello, I'm ".to_owned() + &self.ID;
		let greeting = vec!["HS", &hello];
		self.send_vec(greeting).unwrap();
		self.recv_strings();
		
		let veriKey_vec =  VerifyKey::to_encoded_point(&self.veriKey, true).to_bytes().to_vec();
		self.send(&veriKey_vec).unwrap();
		self.recv_strings();

		Ok(1)
	}

	pub fn key_exchange(&mut self)-> Result<u32, &str> {

		let hello = self.ID.clone() + " sending ECDH public key.";
		self.send_vec(vec!["KE", &hello]);
		self.recv_strings();

		let publicKey_vec = self.publicKey.to_bytes();
		let signature_pk: Signature = self.signKey.sign(&publicKey_vec);

		let mut msg = Vec::new();
		msg.push(signature_pk.as_ref().to_vec());
		msg.push(publicKey_vec.to_vec());

		println!("signature_pk {:?}", signature_pk.as_ref().to_vec());
		println!("publicKey_vec {:?}", publicKey_vec.to_vec());

		self.send_vec(msg).unwrap();
		self.recv_strings(); //server says Ok

		let pk_list = self.recv_broadcast("KE");
		println!("Recieved other's pk: \n {:?}", &pk_list);
		for pk in pk_list{
			let encodedPoint = EncodedPoint::from_bytes(&pk).unwrap();
			let shared: SharedSecret = self.privateKey
				.diffie_hellman(&encodedPoint)
		    	.expect("Some cleint's public key invalid!");
		    self.shareKeys.push(shared);	
		}
		println!("{:?}", self.shareKeys.len());
		Ok(1)
	}
	
	pub fn input_sharing(&mut self, input: &[u8])-> Result<u32, &str> {
		
		let hello = self.ID.clone() + " sharing input.";
		//self.send_vec(vec!["IS", &hello]);
		//self.recv_strings();

		let mut pss = PackedSecretSharing::new_with_param(511, 4, 600);
		for slice in input.iter().collect::<Vec<_>>().chunks(500) {

		}
		Ok(1)
	}


}