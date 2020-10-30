//! Hello World dealer
use zmq::SNDMORE;
use std::str;
use rand_core::{RngCore, OsRng};

use client::*;

fn main() {
	
	let myName = "Cathy";
    let context = zmq::Context::new();
    let mut client = Client::new(&myName, context, "6000", "6001");

    //Key Exhcnage 
    client.handshake();
    client.key_exchange();

	let mut input = Vec::<u8>::new();
	for _ in 0..100 {
		let mut temp = [0u8; 32];
		OsRng.fill_bytes(&mut input);
		input.extend(temp);
	}

	//random input generate
	let mut input = [0u8; 100_000];
	OsRng.fill_bytes(&mut input);
	println!("{:?}", input[9999]);
	client.input_sharing(&input);
}

