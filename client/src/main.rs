//! Hello World dealer
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

    //Key Exhcnage 
	let mut BENCH_TIMER = Instant::now();
    client.handshake().unwrap();
    println!("State 1 elapse {:?}s ({})", BENCH_TIMER.elapsed().as_millis(), myName);
	BENCH_TIMER = Instant::now();

    client.key_exchange().unwrap();
    println!("State 2 elapse {:?}s ({})", BENCH_TIMER.elapsed().as_millis(), myName);

	let mut input = Vec::<u64>::new();
	let mut rng = thread_rng();
	for _ in 0..vectorSize {
		input.push(1);//rng.gen_range(0, u64::MAX));
	}
	BENCH_TIMER = Instant::now();

	// Dropouts
	// if input[0] < u64::MAX/10 {
	// 	panic!("{:?} dropout!", client.ID);
	// }
	client.input_sharing(&mut input).unwrap();
    println!("State 3 elapse {:?}s ({})", BENCH_TIMER.elapsed().as_millis(), myName);
	BENCH_TIMER = Instant::now();

    client.shares_collection().unwrap();
    println!("State 4 elapse {:?}s ({})", BENCH_TIMER.elapsed().as_millis(), myName);
	BENCH_TIMER = Instant::now();

    client.error_correction().unwrap();
    println!("State 5 elapse {:?}s ({})", BENCH_TIMER.elapsed().as_millis(), myName);
	BENCH_TIMER = Instant::now();

    client.aggregation().unwrap();
    println!("State 6 elapse {:?}s ({})", BENCH_TIMER.elapsed().as_millis(), myName);
	BENCH_TIMER = Instant::now();

}


