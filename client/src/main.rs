//! Hello World dealer
use zmq::SNDMORE;
use std::str;
use rand_core::{RngCore, OsRng};

use client::*;

fn main() {
	
	let myName = "Alice";
    let context = zmq::Context::new();
    let mut client = Client::new(&myName, context, "8888", "9999");

    //Key Exhcnage 
    client.handshake();
    client.key_exchange();

	let mut input = Vec::<u64>::new();
	for _ in 0..10000 {
		input.push(OsRng.next_u64());
	}
	println!("{:?}", input[9999]);
	client.input_sharing(&input);
}

