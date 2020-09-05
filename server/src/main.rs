
//! Hello World server in Rust
//! Binds REP socket to tcp://*:5555
//! Expects "Hello" from client, replies with "World"
use std::thread;
use zmq::SNDMORE;


fn main() {
    let context = zmq::Context::new();
    let router = context.socket(zmq::ROUTER).unwrap();

    assert!(router.bind("tcp://*:6000").is_ok());

    loop {
        // First frame in each message is the sender identity
        let identity = router.recv_bytes(0).unwrap();
        if identity.is_empty() {
            break; //  Shutting down, quit
        }

        // Second frame is "fetch" command
        let command = router.recv_string(0).unwrap().unwrap();
        assert!(command == "fetch");

        // Third frame is chunk offset in file
        let offset = router.recv_string(0).unwrap().unwrap();
        let offset = offset.parse::<u64>().unwrap();


        // Send resulting chunk to client
        router.send(&identity, SNDMORE).unwrap();
        router.send("World", 0).unwrap();
    }
}