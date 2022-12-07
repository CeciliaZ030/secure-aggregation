use std::sync::{Arc, Mutex, RwLock};
use anyhow::Result;
use zmq::Socket;

pub struct States {
    clientList: Arc<RwLock<Vec<p256::ecdsa::VerifyingKey>>>,
}


pub struct Worker {
    socket: Socket,
    clientList: Arc<RwLock<Vec<p256::ecdsa::VerifyingKey>>>,

}

impl Worker {
    pub fn new(
        socket: Socket,
        clientList: Arc<RwLock<Vec<p256::ecdsa::VerifyingKey>>>,
    ) -> Self {
        Self {
            socket,
            clientList
        }
    }



    pub fn handshake(&self) -> Result<()> {
        Ok(())
    }
    pub fn key_exchange(&self) -> Result<()> {
        Ok(())
    }
}
