#![allow(unused_imports)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]
#![allow(unused_must_use)]

use std::str;
use std::env;
use std::time::{Duration, Instant};


use zmq::SNDMORE;
use rand_core::{RngCore, OsRng};
use rand::{thread_rng, Rng};


use client::*;

fn main() {

	let args: Vec<String> = env::args().collect();
	
	let myName = &args[1];
	let vectorSize = args[2].parse::<usize>().unwrap();
    let mut client = Client::new(&myName, vectorSize, "8888", "9999");

    let BENCH_TIMER = Instant::now();
    //Key Exhcnage 
    client.handshake().unwrap();
    client.key_exchange().unwrap();
	let mut input = Vec::<u64>::new();
	let mut rng = thread_rng();
	for _ in 0..vectorSize {
		input.push(1);//rng.gen_range(0, u64::MAX));
	}
	// Dropouts
	// if input[0] < u64::MAX/10 {
	// 	panic!("{:?} dropout!", client.ID);
	// }
	client.input_sharing(&mut input).unwrap();
    client.shares_collection().unwrap();
    client.error_correction().unwrap();
    client.aggregation().unwrap();
	println!("Total elapse {:?}ms ({})", BENCH_TIMER.elapsed().as_millis(), myName);

}


