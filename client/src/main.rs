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
    let mut client = Client::new(&myName, vectorSize, "8888", "9999");

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
		input.push(5);
	}
	match client.input_sharing(&input){
    	Ok(_) => (),
    	Err(e) => println!("{:?}", e),
    };	
    match client.aggregation(){
        Ok(_) => (),
        Err(e) => println!("{:?}", e),
    };}


