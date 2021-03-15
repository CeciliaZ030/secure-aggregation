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
    let mut param = Param::new(
            3073700804129980417u128,                // Prime
            1414118249734601779u128, 20,            // Root2, 2^x degree
            308414859194273485u128, 15,             // Root3, 3^x degree
        );
    let mut server = Arc::new(Server::new(
        args[1].parse::<usize>().unwrap(),          // MAX clients
        args[2].parse::<usize>().unwrap(),          // Vector Length
        args[3].parse::<usize>().unwrap(),          // Dropouts
        args[4].parse::<usize>().unwrap(),          // Corrupted Parties
        args[5].parse::<bool>().unwrap(),           // Malicious Flag
        param
    ));

    // Server Thread
    /*
        Runs frontend and backend of zmq sockets structure.
    */
    let ctx = context.clone();
    let svr = server.clone();
    let serverThread = thread::spawn(move || {
        svr.server_task(ctx, 8888);
    });

    // State Thread
    /*
        Recieves information from worker threads,
        constantly keeps track of count and timer,
        changes state once enough client have participated.
    */
    let ctx = context.clone();
    let svr = server.clone();
    let stateThread = thread::spawn(move || {
        match svr.state_task(ctx, 9999, rx) {
            Ok(_) => (),
            Err(e) => println!("{:?}", e),
        };
    });

    // Worker Thread
    /*
        Handles msg passed from the frontend,
        1 msg + 1 reply per loop,
        infoms state thread once successfully process 1 msg.
    */
	let mut workerThreadPool = Vec::new();
	for i in 0..11 {
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