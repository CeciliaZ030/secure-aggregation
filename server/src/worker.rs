use std::collections::HashMap;
use std::str;
use std::{thread, time};
use std::sync::*;
use std::convert::TryInto;
use std::time::Duration;

use zmq::Message;
use p256::ecdsa::VerifyKey;
use crate::Profile;

#[derive(Debug)]
pub enum WorkerError {
	MutexLockFail(usize),
	ClientNotFound(usize),
	ClientExisted(usize),
	MaxClientExceed(usize),
	DecryptionFail(usize),
	UnexpectedFormat(usize),
	UnknownState(usize),
	WrongState(usize),
	SharingFail(usize),
}

#[derive(Debug)]
pub enum ServerError {
	MissingClient(usize),
	MutexLockFail(usize),
	ThreadJoinFail(usize),
	FailPublish(usize),
	UnexpectedField(usize),
	UnknownState(usize),
	TimerFail(usize),
	ThreadSenderFail(usize),
}

pub struct Worker {
	pub ID: String,
	pub dealer : zmq::Socket,
	pub threadSender: mpsc::Sender<usize>,
}

impl Worker {
	pub fn new(ID: &str, 
		context: zmq::Context, threadSender: mpsc::Sender<usize>) -> Worker {
		let dealer = context.socket(zmq::DEALER).unwrap();
		dealer.set_identity(ID.as_bytes());
		assert!(dealer.connect("inproc://backend").is_ok());
		Worker {
			ID: ID.to_string(),
			dealer: dealer,
			threadSender: threadSender,
		}
	}
}

pub fn timer_task(receiver: mpsc::Receiver<usize>, timesUp: Arc<RwLock<bool>>) -> Result<usize, ServerError> {
	let mut T = 0;
	loop {
		if T == 0  {
			T = match receiver.recv() {
				Ok(t) => t,
				Err(_) => {println!("timer recv"); return Err(ServerError::TimerFail(0))},
			};
			println!("Start {}", T);
		} else {
			let mut i = 0;
			while i < T/10 {
				match receiver.try_recv() {
					Ok(t) => {
						T = t;
						println!("Restart {}", T);
						break
					},
					Err(_) =>  continue,//{println!("try_recv"); return Err(ServerError::TimerFail(0))},
				};
				thread::sleep(Duration::from_millis(10));
				i += 1;
			}
			if i == T/10 {
				T = 0;
				match timesUp.write() {
					Ok(mut guard) => *guard = true,
					Err(_) => (),
				};
			}
		}
	}
}

pub fn format_clientData(datas: &mut HashMap<Vec<u8>, Profile>, 
    order: &mut Vec<Vec<u8>>, field: &str) -> Result<Vec<Vec<u8>>, ServerError> {
    /*
		Format to send veriKey or publicKey to clients with clientList order
		Remove from profiles and list if pk is missing
    */
    let mut vecs = Vec::new();
    let mut dropouts = Vec::new();
    for (i, key) in order.iter().enumerate()  {
        match datas.get(key) {
        	Some(d) => {
		        match field {
		        	"veriKey" => {
		        		let vk = VerifyKey::to_encoded_point(&d.veriKey, true).to_bytes().to_vec();
		        		vecs.push(vk)
		        	},
		            "publicKey" => {
		            	if d.publicKey.len() == 0 {
		            		dropouts.push(i);
		            	} else {
		            		vecs.push(d.publicKey.clone());
		            	}
		            },
		            _ => return Err(ServerError::UnexpectedField(0)),
		        };
        	},
        	None => {
        		return Err(ServerError::MissingClient(0))
        	},
        }
    }
   	for i in dropouts {
   		let key = order.remove(i);
   		datas.remove(&key);
   	}
    return Ok(vecs)
}


pub fn read_le_u64(input: &Vec<u8>) -> Vec<u64> {
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
