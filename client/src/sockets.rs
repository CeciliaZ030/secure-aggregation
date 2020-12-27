use std::str;

use zmq::Message;
use zmq::Socket;
use zmq::SNDMORE;

#[derive(Debug, Clone)]
pub enum RecvType {
	bytes(Vec<u8>),
	string(String),
	matrix(Vec<Vec<u8>>),
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
			return RecvType::matrix(data)
		}
	} else {
		return RecvType::string(stringRes)
	}
}

pub fn recv_broadcast(socket: &Socket) -> RecvType {
	let mut data;
	match socket.recv_multipart(0) {
		Ok(msg) => {
				data = msg;
			},
		Err(_) => panic!("Failed to recieve braoadcast."),
	}
	data.remove(0);
	if data.len() == 1 {
		match std::str::from_utf8(&data[0]) {
			Ok(s) => {
				return RecvType::string(s.to_string());
			},
			Err(_) => return RecvType::bytes(data.pop().unwrap()),
		};
		return RecvType::bytes(data.pop().unwrap())
	} else {
		return RecvType::matrix(data);
	}
}

/* data type: 
	Vec<Vec<u8>>, &Vec<Vec<u8>>, [u8] and &[u8] on heap,
	Vec<String>, &Vec<String>, Vec<str>, &Vec<str>
*/
pub fn send_vecs<I, T>(socket: &Socket, data: I) -> Result<&str, &str>
where 
    I: IntoIterator<Item = T>,
    T: Into<Message>,
{

    let result = socket.send_multipart(data, 0);
    match result {
        Ok(T) => Ok("Sent vector successfully."),
        Err(Error) => Err("Failed sending vector."),
    }
}

/* data type: 
	<Vec<u8>, &Vec<u8>, u8 and &u8 on heap,
	String, &String, str, &str
*/    
pub fn send<T>(socket: &Socket, data: T) -> Result<&str, &str>
where
    T: Into<Message>,
{
    let result = socket.send(data, 0);
    match result {
        Ok(T) => Ok("Sent data successfully."),
        Err(Error) => Err("Failed sending data."),
    }
}

