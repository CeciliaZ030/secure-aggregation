use std::str;
use zmq::Message;
use zmq::Socket;
use zmq::SNDMORE;

use crate::*;
use std::sync::*;

#[derive(Debug, Clone)]
pub enum RecvType {
	bytes(Vec<u8>),
	string(String),
	matrix(Vec<Vec<u8>>),
}

pub fn recv(socket: &Socket) -> RecvType {
	let mut data = socket.recv_multipart(0).unwrap();
	if data.len() == 1 && data[0] == Vec::new() {
		return RecvType::bytes(Vec::new());
	}
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

pub fn recv_broadcast(socket: &Socket, topic: &str) -> Result<RecvType, ()> {
	let mut data = match socket.recv_multipart(0) {
		Ok(msg) => msg,
		Err(_) => panic!("Failed to recieve braoadcast."),
	};
	if data.len() == 1 && data[0] == Vec::new() {
		return Ok(RecvType::bytes(Vec::new()));
	}
	let removed = data.remove(0);
	if (removed != topic.as_bytes()) {
		println!("recv_broadcast err {:?}, {:?}", str::from_utf8(&removed), topic);
		return Err(());
	}
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
			return Ok(RecvType::bytes(data.pop().unwrap()));
		} else {
			return Ok(RecvType::matrix(data));
		}
	} else {
		return Ok(RecvType::string(stringRes));
	}
}

pub fn consume_broadcast(socket: &Socket) -> (Vec<u8>, RecvType) {
	let mut data = match socket.recv_multipart(0) {
		Ok(msg) => msg,
		Err(_) => panic!("Failed to recieve braoadcast."),
	};
	let topic = data.remove(0);
	if data.len() == 1 && data[0] == Vec::new() {
		return (topic, RecvType::bytes(Vec::new()));
	}
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
			return (topic, RecvType::bytes(data.pop().unwrap()));
		} else {
			return (topic, RecvType::matrix(data));
		}
	} else {
		return (topic, RecvType::string(stringRes));
	}
}

pub fn sub_task(subscriber: zmq::Socket, 
	buffer: Arc<RwLock<HashMap<Vec<u8>, RecvType>>>, sender: mpsc::Sender<Vec<u64>>) -> Result<usize, ClientError> {
    /*
		Subscriber thread
		Keep recieving from socket
		Consume msg emmited previously, add to buffer if it's new
    */
    loop {
        let (topic, data) = consume_broadcast(&subscriber);
        if buffer.read().unwrap().contains_key(&topic) {
            continue;
        }
        if topic == b"EC" || topic == b"AG" {
        	match data {
        		// m = [[dorpouts], [degree test], [Input Bit test], ....]
        		RecvType::matrix(ref m) => sender.send(read_le_u64(m[0].clone())),
        		_ => return Err(ClientError::UnexpectedRecv(data)),
        	};
        }
        match buffer.write() {
            Ok(mut guard) => guard.insert(topic, data),
            Err(_) => return Err(ClientError::MutexLockFail(0)),
        };
    }
    Ok(0)
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

