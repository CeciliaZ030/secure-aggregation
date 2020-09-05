
//! Hello World server in Rust
//! Binds REP socket to tcp://*:5555
//! Expects "Hello" from client, replies with "World"
use zmq::SNDMORE;


fn main() {

    let context = zmq::Context::new();
    let server = context.socket(zmq::ROUTER).unwrap();

    assert!(server.bind("tcp://*:6000").is_ok());

    let Gx = "48439561293906451759052585252797914202762949526041747995844080717082404635286";
    let Gy = "36134250956749795798585127919587881956611106672985015071877198253568414405109";
    let a = "-3";
    let b = "41058363725152142129326129780047268409114441015993725554835256314039467401291";
    let P = "115792089210356248762697446949407573530086143415290314195533631308867097853951";


    loop {
        // First frame in each message is the sender identity
        let identity = server.recv_bytes(0).unwrap();
        if identity.is_empty() {
            break; //  Shutting down, quit
        }
        println!("{:?} joining", String::from_utf8((&identity).clone()).unwrap());

        let msg = server.recv_string(0).unwrap().unwrap();
        let more_words = server.recv_string(0).unwrap().unwrap();


        // Send resulting chunk to client
        server.send(&identity, SNDMORE).unwrap();
        server.send(&Gx, SNDMORE).unwrap();
        server.send(&Gy, SNDMORE).unwrap();
        server.send(&a, SNDMORE).unwrap();
        server.send(&b, SNDMORE).unwrap();
        server.send(&P, 0).unwrap();
        println!("sent");
    }
}