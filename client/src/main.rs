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
	println!("{:?}", args);
	let V = args[2].parse::<usize>().unwrap();
	let malicious = args[3].parse::<bool>().unwrap();
 
	let mut client;
	match malicious {
		true => {
			// malicious with port
		    if args.len() == 8 {
		        client = Client::new(
		        	&args[1],								// client name
		        	V,								 		// vector size
		        	Some(args[4].parse::<usize>().unwrap()),// input bit limit
		        	Some(&args[5]),							// IP
		        	args[6].parse::<usize>().unwrap(), 		// msg port
		        	args[7].parse::<usize>().unwrap()		// broadcasting port
		        );
		    } 
		    // malicious without port 
		    else if args.len() == 5 {
		        client = Client::new(
		        	&args[1],								// client name
		        	V,								 		// vector size
		        	Some(args[4].parse::<usize>().unwrap()),// input bit limit
		        	None,									// IP
		        	8888, 									// msg port
		        	9999									// broadcasting port
		        );
		    } else {
		    	panic!("Wrong Arguments!");
		    }
		},
		false => {
			// semi-honest with port
		    if args.len() == 7 {
		        client = Client::new(
		        	&args[1],								// client name
		        	V, 										// vector size
		        	None,
		        	Some(&args[4]),							// IP
		        	args[5].parse::<usize>().unwrap(), 		// msg port
		        	args[6].parse::<usize>().unwrap()		// broadcasting port
		        );
		    } 
		    // semi-honest without port
		    else if args.len() == 4 {
		        client = Client::new(
		        	&args[1],								// client name
		        	V, 										// vector size
		        	None,
		        	None,									// IP
		        	8888, 									// msg port
		        	9999									// broadcasting port
		        );
		    } else {
		    	panic!("Wrong Arguments!");
		    }
		},
	}

    let BENCH_TIMER = Instant::now();

    client.handshake().unwrap();
    client.key_exchange().unwrap();

	let mut input = Vec::<u64>::new();
	let mut rng = thread_rng();
	match malicious {
		true => {
			let mut inputBitMod = 0;
			let S = args[4].parse::<usize>().unwrap();
			for i in 0..S {
				inputBitMod += 2u64.pow(i as u32);
			}
			for i in 0..V {
				input.push(rng.gen_range(0, 10) % inputBitMod);
			}
			client.input_sharing_ml(&mut input).unwrap();
		    client.shares_recieving().unwrap();
		    client.error_correction().unwrap();
		},
		false => {
			for i in 0..V {
				input.push(rng.gen_range(0, 10));
			}
			client.input_sharing_sh(&mut input).unwrap();
			client.shares_recieving().unwrap();
		},
	}

	// Dropouts
	if input[0] <= 2 {
		panic!("{:?} dropout!", client.ID);
	}

    client.aggregation().unwrap();
	println!("Total elapse {:?}ms ({})", BENCH_TIMER.elapsed().as_millis(), &args[1]);

}


