use zmq::Message;
use zmq::Socket;
use zmq::SNDMORE;

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
	match std::str::from_utf8(&data[0]) {
		Ok(str) => {
			let mut s = String::new();
			for d in data {
				s.push_str(&String::from_utf8(d).unwrap());
			}
			RecvType::string(s)
		},
		Err(_) => {
			if data.len() == 1 {
				RecvType::bytes(data.pop().unwrap())
			} else {
				RecvType::matrix(data)
			}
		},
	}
}

// Data can be Vec<Vec<u8>> or Vec<String> or Vec<str>
pub fn send_vecs<I, T>(socket: &Socket, data: I, identity: &Vec<u8>) -> Result<usize, usize>
where 
    I: IntoIterator<Item = T> + std::fmt::Debug,
    T: Into<Message>,
{
	println!("send_vecs {:?}", data);
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
	println!("send {:?}", data);
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
	for _ in 0..1_00000 {
		socket.send(topic.as_bytes(), zmq::SNDMORE);
		match socket.send(data.clone(), 0){
	        Ok(T) => continue,
	        Err(Error) => return Err(0),
		 };
	}
	return Ok(0)
}

pub fn publish_vecs<I, T>(socket: &Socket, data: I, topic: &str) -> Result<usize, usize>
where
    I: IntoIterator<Item = T> + std::clone::Clone + std::fmt::Debug,
    T: Into<Message>,
{
    for _ in 0..1_00000 {
		socket.send(topic.as_bytes(), zmq::SNDMORE);
		match socket.send_multipart(data.clone(), 0){
	        Ok(T) => continue,
	        Err(Error) => return Err(0),
		 };
	}
	return Ok(0)
}