use zmq::Message;
use zmq::Socket;
use zmq::SNDMORE;
use std::str;
use std::thread;

use std::thread::sleep;
use std::time::Duration;

#[derive(Debug)]
pub enum RecvType {
	bytes(Vec<u8>),
	string(String),
	matrix(Vec<Vec<u8>>),
}

pub fn take_id(socket: &Socket) -> Vec<u8> {
	socket.recv_bytes(0).unwrap()
}

pub fn recv(socket: &Socket) -> RecvType {
	let mut data = socket.recv_multipart(0).unwrap();
	let mut stringRes = String::new();
	let mut isString = true;
	for d in &data {
		match str::from_utf8(d) {
			Ok(s) => stringRes += s,
			Err(_) => {
				isString = false;
				break
			},
		};
	}
	if !isString {
		if data.len() == 1 {
			return RecvType::bytes(data.pop().unwrap())
		} else {
			//println!("RecvType::matrix(data) {:?}", data.len());
			return RecvType::matrix(data)
		}
	} else {
		return RecvType::string(stringRes)
	}
}

// Data can be Vec<Vec<u8>> or Vec<String> or Vec<str>
pub fn send_vecs<I, T>(socket: &Socket, data: I, identity: &Vec<u8>) -> Result<usize, usize>
where 
    I: IntoIterator<Item = T> + std::fmt::Debug,
    T: Into<Message>,
{
	// println!("send_vecs {:?}", data);
	socket.send(identity, SNDMORE);
    let result = socket.send_multipart(data, 0);
    match result {
        Ok(T) => Ok(0),
        Err(Error) => Err(0),
    }
}

// Data can be Vec<u8> or &str
pub fn send<T>(socket: &Socket, data: T, identity: &Vec<u8>) -> Result<usize, usize>
where
    T: Into<Message> + std::fmt::Debug,
{
	// println!("send {:?} to {:?}", data, String::from_utf8(identity.to_vec()).unwrap());
	socket.send(identity, SNDMORE);
    let result = socket.send(data, 0);
    match result {
        Ok(T) => Ok(0),
        Err(Error) => Err(0),
    }
}

pub fn publish<T>(socket: &Socket, data: T, topic: &str) -> Result<usize, usize>
where
    T: Into<Message> + std::clone::Clone + std::fmt::Debug,
{
	println!("pushing {}", topic);
	for _ in 0..100 {
		socket.send(topic.as_bytes(), zmq::SNDMORE);
		match socket.send(data.clone(), 0){
	        Ok(T) => continue,
	        Err(Error) => return Err(0),
		 };
	}
	sleep(Duration::from_millis(10));
	return Ok(0)
}




pub fn publish_vecs<I, T>(socket: &Socket, data: I, topic: &str) -> Result<usize, usize>
where
    I: IntoIterator<Item = T> + std::clone::Clone + std::fmt::Debug,
    T: Into<Message>,
{
	//println!("+++++++++++++++++++++++++ {:?} data {:?}", topic, data);
    for _ in 0..1004 {
		socket.send(topic.as_bytes(), zmq::SNDMORE);
		match socket.send_multipart(data.clone(), 0){
	        Ok(T) => continue,
	        Err(Error) => return Err(0),
		 };
	}
	sleep(Duration::from_millis(10));
	return Ok(0)
}
