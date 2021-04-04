//! Hello World dealer
use std::str;
use std::env;

use zmq::SNDMORE;
use rand_core::{RngCore, OsRng};
use rand::{thread_rng, Rng};


use client::*;

fn main() {

	let args: Vec<String> = env::args().collect();
	
	let myName = &args[1];
	let vectorSize = args[2].parse::<usize>().unwrap();
    let mut client = Client::new(&myName, vectorSize, "8888", "9999");

    //Key Exhcnage 
    client.handshake().unwrap();
    client.key_exchange().unwrap();
	let mut input = Vec::<u64>::new();
	let mut rng = thread_rng();
	for _ in 0..vectorSize {
		input.push(rng.gen_range(0, 20));
	}
	if input[0] != 10 {
		client.input_sharing(&mut input).unwrap();	
	    client.shares_collection().unwrap();
	    client.error_correction().unwrap();
	    client.aggregation().unwrap();
	}

}


