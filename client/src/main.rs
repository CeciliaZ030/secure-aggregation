//! Hello World client
use zmq::SNDMORE;

fn main() {

    let context = zmq::Context::new();
    let client = context.socket(zmq::DEALER).unwrap();

    let identity = "Alice";
    println!("Alice's name as_bytes {:?}", identity.as_bytes());
    client.set_identity(identity.as_bytes());

    assert!(client.connect("tcp://localhost:6000").is_ok());

    client.send("Hello", SNDMORE).unwrap();
    client.send("blah", 0).unwrap();

    let Gx = client.recv_string(0).unwrap().unwrap();
    let Gy = client.recv_string(0).unwrap().unwrap();
    let a = client.recv_string(0).unwrap().unwrap();
    let b = client.recv_string(0).unwrap().unwrap();
    let P = client.recv_string(0).unwrap().unwrap();

    println!("{}, {}, {}, {}, {}", Gx, Gy, a, b, P);
}