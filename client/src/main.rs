//! Hello World client

fn main() {
    println!("Connecting to hello world server...\n");

    let context = zmq::Context::new();
    let client = context.socket(zmq::REQ).unwrap();
    let identity = "Alice";
    println!("Alice's name as_bytes {:?}", identity.as_bytes());
    client.set_identity(identity.as_bytes());

    assert!(client.connect("tcp://localhost:8888").is_ok());

    let mut msg = zmq::Message::new();

    /* 		Handshake	 */

    client.send("Hello", 0).unwrap();
    

    let Gx = client.recv_string(0).unwrap().unwrap();
    let Gy = client.recv_string(0).unwrap().unwrap();
    let a = client.recv_string(0).unwrap().unwrap();
    let b = client.recv_string(0).unwrap().unwrap();
    let P = client.recv_string(0).unwrap().unwrap();

    println!("{}, {}, {}, {}, {}", Gx, Gy, a, b, P);

}