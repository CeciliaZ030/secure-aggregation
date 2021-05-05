#![allow(unused_imports)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]
#![allow(unused_must_use)]

use std::str;
use std::sync::*;
use std::thread;
use std::env;

use zmq;
use server::*;
use server::param::*;
use server::worker::*;

fn main() {

    let args: Vec<String> = env::args().collect();
	println!("hello");
    let context = zmq::Context::new();
    let (tx, rx) = mpsc::channel();
    let param = Param::new(
            3073700804129980417u64,                // Prime 62 bits
            1414118249734601779u64, 20,            // Root2, 2^x degree
            308414859194273485u64, 15,             // Root3, 3^x degree
        );
    /*
        May provide IP addr & two different ports# 
    */
    assert!(args.len() == 9 || args.len() == 12);   
    let server = Arc::new(Server::new(
        args[1].parse::<usize>().unwrap(),          // MAX clients
        args[2].parse::<usize>().unwrap(),          // Vector Length
        args[3].parse::<usize>().unwrap(),          // Input Bit Limit
        args[4].parse::<usize>().unwrap(),          // Dropouts
        args[5].parse::<usize>().unwrap(),          // Session time
        args[6].parse::<usize>().unwrap(),          // IS Session time
        args[7].parse::<usize>().unwrap(),           // Corrupted Parties
        args[8].parse::<bool>().unwrap(),           // Malicious Flag
        param
    ));
    let primes = vec![2,3,5,7,11,13,17,19,23,29,31,37,41,43,47,53,59,61,67,71,
    73,79,83,89,97,101,103,107,109,113,127,131,137,139,149,151,157,163,167,173,
    179,181,191,193,197,199,211,223,227,229,233,239,241,251,257,263,269,271,277,
    281,283,293,307,311,313,317,331,337,347,349,353,359,367,373,379,383,389,397,
    401,409,419,421,431,433,439,443,449,457,461,463,467,479,487,491,499,503,509,
    521,523,541,547,557,563,569,571,577,587,593,599,601,607,613,617,619,631,641,
    643,647,653,659,661,673,677,683,691,701,709,719,727,733,739,743,751,757,761,
    769,773,787,797,809,811,821,823,827,829,839,853,857,859,863,877,881,883,887,
    907,911,919,929,937,941,947,953,967,971,977,983,991,997,1009];
    if primes.contains(&args[2].parse::<usize>().unwrap()) {
        panic!("vector length is prime!");
    }

    // Server Thread
    /*
        Runs frontend and backend of zmq sockets structure.

        Reciever port: arg[10] 
        (defualt: 8888)
    */
    let ctx = context.clone();
    let svr = server.clone();
    let arg = args.clone();
    let serverThread = thread::spawn(move || {
        if arg.len() == 12 {
             svr.server_task(ctx, Some(&arg[9]),  arg[10].parse::<usize>().unwrap());
        } else {
            svr.server_task(ctx, None, 8888);
        }
    });

    // State Thread
    /*
        Recieves information from worker threads,
        constantly keeps track of count and timer,
        changes state once enough client have participated.

        Publisher port: arg[11]
        (default: 9999)
    */
    let ctx = context.clone();
    let svr = server.clone();
    let arg = args.clone();
    let stateThread = thread::spawn(move || {
        if arg.len() == 12 {
             match svr.state_task(ctx, Some(&arg[9]), arg[11].parse::<usize>().unwrap(), rx) {
                Ok(_) => (),
                Err(e) => println!("{:?}", e),
            };
        } else {
            match svr.state_task(ctx, None, 9999, rx) {
                Ok(_) => (),
                Err(e) => println!("{:?}", e),
            };
        }
    });

    // Worker Thread
    /*
        Handles msg passed from the frontend,
        1 msg + 1 reply per loop,
        infoms state thread once successfully process 1 msg.
    */
	let mut workerThreadPool = Vec::new();
	for i in 0..32 {
		let worker = Worker::new(
            &format!("Worker{}", i.to_string()),
            context.clone(), 
            tx.clone()
        );
        let svr = server.clone();
    	let child = thread::spawn(move || {
    		println!("spawning {:?}", i);
            match svr.worker_task(worker) {
                Ok(_) => (),
                Err(e) => println!("{:?}", e),
            };
	    });
	    workerThreadPool.push(child);
    }


    for wt in workerThreadPool {
    	wt.join().unwrap();
    }
    serverThread.join().unwrap();
    stateThread.join().unwrap();

    println!("Application shut down.");

}
