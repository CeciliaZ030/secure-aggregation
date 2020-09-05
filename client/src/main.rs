//! Hello World client
use zmq::SNDMORE;

fn main() {

    let context = zmq::Context::new();
    let dealer = context.socket(zmq::DEALER).unwrap();

    let identity = "Alice";
    println!("Alice's name as_bytes {:?}", identity.as_bytes());
    dealer.set_identity(identity.as_bytes());

    assert!(dealer.connect("tcp://localhost:6000").is_ok());

    loop {

        dealer.send("fetch", SNDMORE).unwrap();
        dealer.send("645", 0).unwrap();

        let chunk = dealer.recv_string(0).unwrap().unwrap();
        println!("{:?} chunks received", chunk);
    }

}