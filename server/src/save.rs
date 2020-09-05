//! Hello World server

use std::thread;
use std::time::Duration;
use zmq::SNDMORE;

fn main() {

    let context = zmq::Context::new();
    let server = context.socket(zmq::ROUTER).unwrap();

    assert!(server.bind("tcp://*:8888").is_ok());

    let Gx = "48439561293906451759052585252797914202762949526041747995844080717082404635286";
    let Gy = "36134250956749795798585127919587881956611106672985015071877198253568414405109";
    let a = "-3";
    let b = "41058363725152142129326129780047268409114441015993725554835256314039467401291";
    let P = "115792089210356248762697446949407573530086143415290314195533631308867097853951";

    let mut STATE = 0;
    let mut client_list: Vec<String> = vec![];

    loop {
    	match STATE {
    		0 => {
				// First frame in each message is the sender identity
		        let identity = server.recv_bytes(0).unwrap();
		        if identity.is_empty() {
		            break;
		        }

			    let msg = server.recv_string(0).unwrap().unwrap();
			    println!("Recieved {:?} from {}", msg, String::from_utf8(identity).unwrap());
			    thread::sleep(Duration::from_millis(1000));
				server.send(&Gx, SNDMORE).unwrap();
				server.send(&Gy, SNDMORE).unwrap();
				server.send(&a, SNDMORE).unwrap();
				server.send(&b, SNDMORE).unwrap();
				server.send(&P, 0).unwrap();
				println!("sent");

    		},
    		1 => {
    			unimplemented!();
    		},
    		_=> {
    			unimplemented!();
    		},
    	}
    }

}