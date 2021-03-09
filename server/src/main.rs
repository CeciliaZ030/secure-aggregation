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
            3073700804129980417u128, 
            1414118249734601779u128, 20,
            308414859194273485u128, 15,
        );
    let mut server = Arc::new(Server::new(
        args[1].parse::<usize>().unwrap(),          // MAX clients
        args[2].parse::<usize>().unwrap(), param)   // Vecor length
    );

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
        constantly keeps track of count,
        changes state once enough client have participated.
    */
    let ctx = context.clone();
    let svr = server.clone();
    let stateThread = thread::spawn(move || {
        svr.state_task(ctx, 9999, rx);
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
	        svr.worker_task(worker);
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