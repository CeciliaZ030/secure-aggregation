use std::convert::TryInto;
use zmq::Message;
use p256::ecdsa::VerifyKey;


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
    SharingFail(usize)
}

#[derive(Debug)]
pub enum ServerError {
    MissingClient(usize),
    MutexLockFail(usize),
    ThreadJoinFail(usize),
    FailPublish(usize),
    UnexpectedField(usize),
}

pub struct Worker {
	ID: String,
	dealer : zmq::Socket,
	threadSender: mpsc::Sender<usize>,
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

pub fn format_clientData(datas: &HashMap<Vec<u8>, Profile>, 
    order: &Vec<Vec<u8>>, field: &str) -> Result<Vec<Vec<u8>>, ServerError> {
    //println!("order {:?}", order);
    let mut vecs = Vec::new();
    for key in order {
        let data = datas.get(key);
        match data {
        	Some(d) => {
		        let res = match field {
		        	"veriKey" => VerifyKey::to_encoded_point(&d.veriKey, true)
		        					.to_bytes()
		        					.to_vec(),
		            "publicKey" => d.publicKey.clone(),
		            _ => return Err(ServerError::UnexpectedField(0)),
		        };
		        vecs.push(res);
        	},
        	None => {
        		return Err(ServerError::MissingClient(0))
        	},
        }
    } 
    return Ok(vecs)
}

fn read_le_u64(input: &Vec<u8>) -> Vec<u64> {
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