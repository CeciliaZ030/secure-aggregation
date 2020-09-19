//! Hello World dealer
use zmq::SNDMORE;
use rug::{Assign, Integer};
use rug::{rand::RandState};
use std::str;

#[derive(Debug)]
struct PublicInfo {
    Gx: Integer,
    Gy: Integer,
    a: Integer,
    b: Integer,
    P: Integer,
}

fn main() {

    let context = zmq::Context::new();
    let dealer = context.socket(zmq::DEALER).unwrap();


    let identity = "Alice";
    println!("Alice's name as_bytes {:?}", identity.as_bytes());
    dealer.set_identity(identity.as_bytes());

    assert!(dealer.connect("tcp://localhost:6000").is_ok());


    dealer.send("KE", SNDMORE).unwrap();
    dealer.send("and more..", 0).unwrap();

    let msg = dealer.recv_multipart(0).unwrap();

    for m in msg {
        println!("{}",str::from_utf8(&m).unwrap());
    }
    dealer.send("234023478349538459", 0).unwrap();

    let subscriber = context.socket(zmq::SUB).unwrap();
    subscriber
        .connect("tcp://localhost:6001")
        .expect("failed connecting subscriber");
    subscriber.set_subscribe(b"Server Broadcast").expect("failed subscribing");

    let envelope = subscriber
        .recv_string(0)
        .expect("failed receiving envelope")
        .unwrap();
    let message = subscriber
        .recv_multipart(0)
        .expect("failed receiving message");
    println!("[{}]", envelope);
    for m in message {
        println!("{}",str::from_utf8(&m).unwrap());
    }

    // dealer.send("blah blah", SNDMORE).unwrap();
    // dealer.send("b ehm", 0).unwrap();
    // println!("sent");
    // let Gx = dealer.recv_string(0).unwrap().unwrap();
    // let Gy = dealer.recv_string(0).unwrap().unwrap();
    // let a = dealer.recv_string(0).unwrap().unwrap();
    // let b = dealer.recv_string(0).unwrap().unwrap();
    // let P = dealer.recv_string(0).unwrap().unwrap();

    //println!("public params \n {}, {}, {}, {}, {}", Gx, Gy, a, b, P);

    // let publicInfo = PublicInfo {
    //     Gx: Gx.parse::<Integer>().unwrap(),
    //     Gy: Gy.parse::<Integer>().unwrap(),
    //     a: a.parse::<Integer>().unwrap(),
    //     b: b.parse::<Integer>().unwrap(),
    //     P: P.parse::<Integer>().unwrap(),
    // };

    // println!("#bits of P {:?}", publicInfo.P.significant_bits());

    // /*      Key Generation      */

    // // random private key
    // let mut rand = RandState::new();
    // let sk = Integer::from(100);//publicInfo.P.clone().random_below(&mut rand);

    // //comput public key by Eliptic Curve addition
    // let pk = EC_mul(&publicInfo, &sk);

    // dealer.send(pk.to_string_radix(10), 0).unwrap();
    // println!("Sent sk {:?}, pk {:?}", sk, pk);

}

fn EC_mul(pi: &PublicInfo, sk: &Integer) -> (Integer, Integer) {

    let mut xR = pi.Gx.clone();
    let mut yR = pi.Gx.clone();
    let mut _sk = sk.clone();

    while _sk > 1 {

        let slope : Integer = (3 * xR.clone().square() + pi.a.clone()) / (2 * yR.clone());

        xR = slope.clone().square() - 2 * xR.clone();

        let diff = xR.clone() - yR.clone();
        yR = slope * diff - yR.clone();
        _sk -= 1;
    }

    return (xR, yR)
}
