use std::collections::HashMap;
use std::ops::DerefMut;
use std::sync::{Arc, RwLock};
use std::thread::{JoinHandle, spawn};
use anyhow::Result;
use zmq::{Context, Message, Socket};
use router::server::ServerRouter;
use router::server::*;
use router::types::*;

mod worker;
use worker::*;
use crate::ServerState::*;

const NUM_WORKERS: u64 = 8;
const MAX_CLIENTS: u64 = 8;


pub struct Server {
    context: Context,
    router: ServerRouter,
    workers: Vec<JoinHandle<Result<()>>>,
    state: Arc<RwLock<ServerState>>,
    client_profiles: Arc<RwLock<HashMap<ID, Profile>>>,

}

#[derive(Default, Clone)]
pub struct Profile {
    pub auth_key: Option<p256::ecdsa::VerifyingKey>,
    pub ecdh_pubkey: Option<p256::PublicKey>,
    pub shares: Option<Vec<u8>>
}

#[derive(Default, Clone)]
pub enum ServerState {
    #[default]
    Default,
    Handshake,
    KeyExchange,
    InputSharing
}

impl Server {
    pub fn new() -> Result<Self> {
        let context = zmq::Context::new();
        let router = ServerRouter::new(context.clone())?;
        let state = ServerState::default();
        let client_profiles = HashMap::<ID, Profile>::new();
        let workers = Vec::new();
        let mut server = Server {
            context,
            router,
            workers,
            state: Arc::new(RwLock::new(state)),
            client_profiles: Arc::new(RwLock::new(client_profiles)),
        };
        Ok(server)
    }

    pub fn init(&mut self) -> Result<()> {
        use ServerState::*;
        *self.state.write().unwrap() = Handshake;
        self.spawn_workers()?;
        loop {
            let cur_state = &*self.state.read().unwrap();
            match cur_state {
                Handshake => self.handshake_transit()?,
                KeyExchange => self.keyexchange_transit()?,
                _ => {}
            };
        }
    }

    pub fn spawn_workers(&mut self) -> Result<Vec<JoinHandle<Result<()>>>> {
        let mut threads = Vec::new();
        for _ in 0..NUM_WORKERS {
            let ctx = self.context.clone();
            let state = self.state.clone();
            let client_profiles = self.client_profiles.clone();
            let t = spawn(move || -> Result<()> {
                let socket = ctx.socket(zmq::REP)?;
                socket.connect(format!("{}{}", LOCALHOST, PORT_PUB).as_str())?;
                let worker = Worker::new(ctx, client_profiles)?;
                loop {
                    worker.run_task(&*state.read().unwrap())?;
                }
            });
            threads.push(t);
        }
        Ok(threads)
    }

    pub fn handshake_transit(&self) -> Result<()> {
        if self.client_profiles.read().unwrap().len() >= MAX_CLIENTS as usize {
            *self.state.write().unwrap() = KeyExchange;
        }
        Ok(())
    }

    pub fn keyexchange_transit(&self) -> Result<()> {
        todo!()
    }

}
