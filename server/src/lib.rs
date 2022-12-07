mod worker;

use std::sync::{Arc, Mutex, RwLock};
use std::thread::{JoinHandle, spawn};
use anyhow::{anyhow, Result};
use zmq::{Context, Message, Socket};
use router::server::ServerRouter;
use router::types::ServerMessageType;
use crate::worker::Worker;


pub struct Server {
    clientList: Arc<RwLock<Vec<p256::ecdsa::VerifyingKey>>>,
    context: Context,
    router: ServerRouter,
    workers: Vec<JoinHandle<Result<()>>>,
}

impl Server {
    pub fn new() -> Self {
        let clientList = Vec::<p256::ecdsa::VerifyingKey>::new();
        let mut server = Self {
            clientList: Arc::new(RwLock::new(clientList)),
            context: Context::new(),
            router: ServerRouter::new(10)?,
            workers: Vec::new()
        };
        server.run_workers(8);
    }

    pub fn run_workers(&mut self, num: u64) -> Result<()> {
        let ctx = self.context.clone();
        for _ in 0..num {
            let handle = spawn(|| -> Result<()> {
                let socket = ctx.socket(zmq::REP)?;
                socket.bind(format!("{:?}{:?}", INPROC, PORT1).as_str())?;
                let worker = Worker::new(
                    socket,
                    self.clientList.clone()
                );
                loop {
                    let id = worker.recv_msg(zmq::DONTWAIT)?;
                    let (msg_type, msgs) = Self::_receive(&worker)?;
                    match msg_type {
                        ServerMessageType::Handshake => handshake(),
                        ServerMessageType::KeyExchange => Ok(Message::from("KE")),
                        _ => anyhow!("Unknown server message type")
                    }
                }
                Ok(())
            });
            self.workers.push(handle);
        }
        Ok(())
    }

}
