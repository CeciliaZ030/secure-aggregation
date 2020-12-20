
//! Hello World server in Rust
//! Binds REP socket to tcp://*:5555
//! Expects "Hello" from client, replies with "World"
use zmq::SNDMORE;
use std::str;

use server::*;


fn main() {

    println!("hello");
    let context = zmq::Context::new();
    let mut server = Server::new(context, "6000", "6001");

    loop {

        let identity = server.take_id();
        println!("Taken {:?}", String::from_utf8(identity.clone()).unwrap());

        let msg = server.recv_strings();
        
        server.workflow(identity, msg);

    }
}


