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
	let inputBitLimit = args[3].parse::<usize>().unwrap();
    let mut client = Client::new(myName, vectorSize, inputBitLimit, "8888", "9999");

    let BENCH_TIMER = Instant::now();

    client.handshake().unwrap();
    client.key_exchange().unwrap();

	let mut input = Vec::<u64>::new();
	let mut inputBitMod = 0;
	for i in 0..inputBitLimit {
		inputBitMod += 2u64.pow(i as u32);
	}
	let mut rng = thread_rng();
	for i in 0..vectorSize {
		//input.push(20 % inputBitMod);
		input.push(rng.gen_range(0, 3) % inputBitMod);
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


