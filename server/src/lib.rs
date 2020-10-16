use std::collections::HashMap;
use zmq::Message;
use zmq::SNDMORE;
use std::str;

// use ecdsa::{verify::VerifyKey, 
//             Signature, signature::Signer,
//             hazmap::VerifyPrimitive,
//             };

use signature::Signature as _;
use p256::NistP256;
use p256::{
    ecdsa::{SigningKey, Signature, signature::Signer},
    ecdsa::{VerifyKey, signature::Verifier},
};

static MAX_USER: usize = 512;

pub struct Server{

	pub reciever: zmq::Socket,
	pub publisher: zmq::Socket,
	
	state: u32,
	counter: u32,

	client_list: Vec<Vec<u8>>,
	client_data: HashMap<Vec<u8>, ClientData>,
}

#[derive(Debug)]
pub struct ClientData {
	veriKey: Vec<u8>,
	publicKey:  Vec<u8>,
}


impl Server {
	
	pub fn new(context: zmq::Context, port1: &str, port2: &str) -> Server {

		let reciever = context.socket(zmq::ROUTER).unwrap();
		let publisher = context.socket(zmq::PUB).unwrap();

//HWM
//Buffer size
    //sending & recving

        publisher.set_sndhwm(1_100_000).expect("failed setting hwm");
		let mut addr1: String = "tcp://*:".to_owned();
		let mut addr2: String = "tcp://*:".to_owned();
		addr1.push_str(port1);
		addr2.push_str(port2);
		assert!(reciever.bind(&addr1).is_ok());
		assert!(publisher.bind(&addr2).is_ok());

		Server {

			reciever: reciever,
			publisher: publisher,

			state: 0u32,
			counter: 0u32,

			client_list: Vec::<Vec<u8>>::new(),
			client_data: HashMap::<Vec<u8>, ClientData>::new(),
		}
	}

	pub fn take_id(&self) -> Vec<u8> {
		self.reciever.recv_bytes(0).unwrap()
	}
	
	pub fn recv_strings(&self) -> Vec<String> {
		let mut res : Vec<String> = Vec::new();
		let msg = self.reciever.recv_multipart(0).unwrap();
        for m in msg {
            res.push(String::from_utf8(m).unwrap());
        }
        println!("recv_strings {:?}", res);
        res
	}
	pub fn recv_vecs(&self) -> Vec<Vec<u8>> {
		let msg = self.reciever.recv_multipart(0).unwrap();
		println!("recv_vecs {:?}", msg);
		msg
	}
	//pub fn recv_multipart(&self, flags: i32) -> Result<Vec<Vec<u8>>>
    //pub fn recv_string(&self, flags: i32) -> Result<result::Result<String, Vec<u8>>> 
    //pub fn recv_bytes(&self, flags: i32) -> Result<Vec<u8>> 
	//pub fn recv_into(&self, bytes: &mut [u8], flags: i32) -> Result<usize>
	
   
    // Data can be Vec<Vec<u8>> or Vec<String> or Vec<str>
    pub fn send_vec<I, T>(&self, data: I, identity: Vec<u8>) -> Result<&str, &str>
    where 
        I: IntoIterator<Item = T>,
        T: Into<Message>,
    {
    	self.reciever.send(identity, SNDMORE);
        let result = self.reciever.send_multipart(data, 0);
        match result {
            Ok(T) => Ok("Sent vector successfully."),
            Err(Error) => Err("Failed sending vector."),
        }
    }

    // Data can be Vec<u8> or &str
    pub fn send<T>(&self, data: T, identity: &Vec<u8>) -> Result<&str, &str>
    where
        T: Into<Message>,
    {
    	self.reciever.send(identity, SNDMORE);
        let result = self.reciever.send(data, 0);
        match result {
            Ok(T) => Ok("Sent message successfully."),
            Err(Error) => Err("Failed sending message."),
        }
    }

    // Data can be Vec<Vec<u8>> or Vec<String> or Vec<str>
    pub fn publish_vec(&self, data: Vec<Vec<u8>>, topic: &str){
        for _ in 0..1_000 {
            self.publisher.send(topic.as_bytes(), zmq::SNDMORE);
            self.publisher
                .send_multipart(&data, 0)
                .expect("Failed publishing vector");
        }
        self.publisher.send("END", 0).expect("failed sending end");
    
    }

    pub fn publish<T>(&self, data: T) -> Result<&str, &str>
    where
        T: Into<Message>,
    {
        let result = self.publisher.send(data, 0);
        match result {
            Ok(T) => Ok("Published message successfully."),
            Err(Error) => Err("Failed publishing message."),
        }
    }

	pub fn workflow(&mut self, identity: Vec<u8>, msg: Vec<String>) -> Result<u32, &str>
	{

		match msg[0].as_str() {
    		"HS" => {
            //Registration
    			if self.client_list.contains(&identity){
                    self.send("Error: existed client.", &identity);
    				return Err("Handshake: client already exists.")
    			}
    			if self.client_list.len() == 3 {
                    self.send("Error: reached maximun client number.", &identity);
    				return Err("Handshake: reached maximun client number.")
    			}
    			self.client_list.push(identity.clone());
		        self.send("Hello, I'm server.", &identity);
    			
    			// Recieve client's veryfying key
    			assert_eq!(&self.take_id(), &identity);
    			let veriKey = self.recv_vecs();
    			let data = ClientData {
    				veriKey: veriKey[0].clone(),
    				publicKey: Vec::<u8>::new(),
    			};
    			self.client_data.insert(identity.clone(), data);

				self.send("OK", &identity);

				Ok(1)
    		},
    		"KE" => {
    			self.send("Send your publicKey", &identity);
                assert_eq!(&self.take_id(), &identity);
    			let authenticated_pk = self.recv_vecs();

                let pk = &authenticated_pk[1];
                let signature = Signature::from_bytes(&authenticated_pk[0]).unwrap();
                
                let client_data = self.client_data.get_mut(&identity).unwrap();
                let veriKey = VerifyKey::new(
                                &client_data.veriKey
                              ).unwrap();
                
                let verification = veriKey.verify(pk, &signature);
                match verification {
                    Ok(_) => {
                        client_data.publicKey = pk.to_vec();
                        self.send("Your publicKey has been save.", &identity);
                        self.counter += 1;
                    },
                    Err(_) => {
                        self.send("Error: public key authentication failed.", &identity);
                        return Err("Key Exchange: public key authentication failed.");
                        },
                }
                if self.counter == 3 {
                    let msgVec = client_data_to_vector(&self.client_data, &self.client_list, "publicKey");
                    println!("Publishing \n {:?}", msgVec);
                    self.publish_vec(msgVec, "KE");
                    self.counter = 0;
                }
                Ok(1)
    		},
    		"IS" => {

    			Ok(1)
    		},
    		_ => {
    			panic!("What the heck??")
    		}
    	}
	}

}

pub fn client_data_to_vector(datas: &HashMap<Vec<u8>, ClientData>, order: &Vec<Vec<u8>>, field: &str) -> Vec<Vec<u8>> {
    
    let mut vector = Vec::new();
    for key in order {
        let data = datas.get(key).unwrap();
        let res = match field {
            "veriKey" => data.veriKey.clone(),
            "publicKey" => data.publicKey.clone(),
            _ => panic!("Unknow field"),
        };
        vector.push(res);
    } 
    vector
}






