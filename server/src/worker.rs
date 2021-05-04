use std::collections::HashMap;
use std::str;
use std::{thread, time};
use std::sync::*;
use std::convert::TryInto;
use std::time::Duration;

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
					Err(e) => {
						thread::sleep(Duration::from_millis(10));
						i += 1
					},
				};
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

pub fn format_clientData(profiles: &mut HashMap<Vec<u8>, Profile>, 
    list: &mut Vec<Vec<u8>>, field: &str) -> Result<Vec<Vec<u8>>, ServerError> {
    /*
		Format to send veriKey or publicKey to clients with clientList list
		Remove from profiles and list if pk is missing
    */
    let mut vecs = Vec::new();
    let mut dropouts = Vec::new();
    for (i, key) in list.iter().enumerate()  {
        match profiles.get(key) {
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
   		let key = list.remove(i);
   		profiles.remove(&key);
   	}
    return Ok(vecs)
}


pub fn write_u64_le_u8(v: &[u64]) -> &[u8] {
	/*
		Write u64 integer array into continuous bytes array
	*/
    unsafe {
        std::slice::from_raw_parts(
            v.as_ptr() as *const u8,
            v.len() * std::mem::size_of::<u64>(),
        )
    }
}

pub fn write_usize_le_u8(v: &[usize]) -> &[u8] {
	/*
		Write usize integer array into continuous bytes array
	*/
    unsafe {
        std::slice::from_raw_parts(
            v.as_ptr() as *const u8,
            v.len() * std::mem::size_of::<usize>(),
        )
    }
}

pub fn read_le_u64(input: &Vec<u8>) -> Vec<u64> {
	/*
		Read little endian bytes Vec<u8> of u64 integer array
		back to Vec<u64>
	*/
    let mut res = Vec::<u64>::new();
    if input.len() == 0 {
    	return res;
    }
    let mut ptr = &mut input.as_slice();
    loop {
        let (int_bytes, rest) = ptr.split_at(std::mem::size_of::<u64>());
        *ptr = rest;
        res.push(u64::from_le_bytes(int_bytes.try_into().unwrap()));
        if rest.len() < 8 {
            break;
        }
    }
    res
}

pub fn read_le_usize(input: &Vec<u8>) -> Vec<u64> {
	/*
		Read little endian bytes Vec<u8> of usize integer array
		back to Vec<usize>
	*/
    let mut res = Vec::<u64>::new();
    if input.len() == 0 {
    	return res;
    }
    let mut ptr = &mut input.as_slice();
    loop {
        let (int_bytes, rest) = ptr.split_at(std::mem::size_of::<u64>());
        *ptr = rest;
        res.push(u64::from_le_bytes(int_bytes.try_into().unwrap()));
        if rest.len() < 8 {
            break;
        }
    }
    res
}


