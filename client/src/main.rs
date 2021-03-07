//! Hello World dealer
use std::str;
use std::env;

use zmq::SNDMORE;
use rand_core::{RngCore, OsRng};

use client::*;

fn main() {

	let args: Vec<String> = env::args().collect();
	
	let myName = &args[1];
	let vectorSize = args[2].parse::<usize>().unwrap();
    let context = zmq::Context::new();
    let mut client = Client::new(&myName, context, "8888", "9999");

    //Key Exhcnage 
    match client.handshake(){
    	Ok(_) => (),
    	Err(e) => println!("{:?}", e),
    };
    match client.key_exchange(){
    	Ok(_) => (),
    	Err(e) => println!("{:?}", e),
    };

	let mut input = Vec::<u64>::new();
	for _ in 0..vectorSize {
		input.push(OsRng.next_u64());
	}
	match client.input_sharing(&input){
    	Ok(_) => (),
    	Err(e) => println!("{:?}", e),
    };	
	client.input_sharing(&input);
}


