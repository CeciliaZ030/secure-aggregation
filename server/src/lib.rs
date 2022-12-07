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
    pub fn handshake(&self) -> Result<()> {
        Ok(())
    }
    pub fn key_exchange(&self) -> Result<()> {
        Ok(())
    }
}

pub fn mainn() {

}

pub trait MessageHandler {
    /// Handshake with incoming client
    fn handle_handshake(&self) -> Result<()>;
    /// Key Exchange
    fn handle_key_exchange(&self) -> Result<()>;
    /// Input shareing
    fn handle_input_sharing(&self) -> Result<()>;
    /// Correct error based on client response
    fn handle_error_correction(&self) -> Result<()>;
}
