//! Hello World dealer
use std::str;
use std::env;

use zmq::SNDMORE;
use rand_core::{RngCore, OsRng};

use client::*;

fn main() {

	let args: Vec<String> = env::args().collect();
	
	let myName = &args[1];
    let context = zmq::Context::new();
    let mut client = Client::new(&myName, context, "8888", "9999");

    //Key Exhcnage 
    client.handshake();
    client.key_exchange();

	let mut input = Vec::<u64>::new();
	for _ in 0..80 {
		input.push(OsRng.next_u64());
	}
	client.input_sharing(&input);
}

