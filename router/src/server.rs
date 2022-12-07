use std::collections::HashMap;
use std::fmt::format;
use std::convert::*;
use std::ops::Deref;
use std::sync::mpsc;
use std::thread::{JoinHandle, spawn};
use anyhow::{Error, Result, anyhow};
use p256;
use aes_gcm;
use p256::ecdsa::signature::Signature;
use p256::ecdsa::VerifyingKey;
use p256::elliptic_curve::AffinePoint;
use p256::elliptic_curve::group::GroupEncoding;
use p256::NistP256;
use zmq::{Context, Socket, Message};
use crate::types::{ID, ServerMessage, ServerMessageType, State};


const LOCALHOST: &'static str = "tcp://localhost:";
const INPROC: &'static str = "inproc://backend:";
const PORT1: u64 = 7777;
const PORT2: u64 = 7676;
const PORT3: u64 = 7878;

pub struct ServerRouter {
    context: Context,
    frontend: Socket,
    backend: Socket,
    publisher: Socket,
    workers: Vec<JoinHandle<Result<()>>>,

    tx_pool: HashMap<String, HashMap<State, Vec<Message>>>,
}

impl ServerRouter {

    pub fn new(worker_num: u64) -> Result<Self> {
        let context = Context::new();
        let frontend = context.socket(zmq::ROUTER)?;
        let backend = context.socket(zmq::DEALER)?;
        let publisher = context.socket(zmq::PUB)?;

        frontend.bind(format!("{:?}{:?}", LOCALHOST, PORT1).as_str())?;
        backend.bind(format!("{:?}{:?}", INPROC, PORT3).as_str())?;
        publisher.bind(format!("{:?}{:?}", INPROC, PORT2).as_str())?;

        zmq::proxy(&frontend, &backend)?;

        let ctx = context.clone();
        let mut workers = Vec::new();

        Ok(Self {
            context,
            frontend,
            backend,
            publisher,
            workers,
            tx_pool: HashMap::<String, HashMap<State, Vec<Message>>>::new(),
        })
    }


    fn _send(worker: &Socket, id: ID, msg: Vec<Message>) -> Result<()> {
        worker.send(id, zmq::SNDMORE);
        for m in msg {
            worker.send(m, 0)?;
        }
        Ok(())
    }

    fn _receive(worker: &Socket) -> Result<(ServerMessageType, Vec<Message>)> {
        let mut msgs = Vec::<Message>::new();
        while worker.get_rcvmore()? {
            msgs.push(worker.recv_msg(0)?);
        }
        let ty = msgs[0].try_into()?;
        Ok((ty, msgs[1..].to_vec()))
    }

}


