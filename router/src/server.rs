use std::collections::HashMap;
use std::fmt::format;
use std::convert::*;
use std::ops::Deref;
use std::sync::mpsc;
use anyhow::{Error, Result, anyhow};
use p256;
use aes_gcm;
use p256::ecdsa::signature::Signature;
use p256::ecdsa::VerifyingKey;
use p256::elliptic_curve::AffinePoint;
use p256::elliptic_curve::group::GroupEncoding;
use p256::NistP256;
use zmq::{Context, Socket, Message};
use crate::ClientMessage::*;

const LOCALHOST: &'static str = "tcp://localhost:";
const PORT1: u64 = 7777;
const PORT2: u64 = 7676;

pub struct ClientRouter {
    context: Context,
    send_recv: Socket,
    pub_sub: Socket,
    subscriptions: HashMap<String, Option<Vec<Message>>>,
    notifyer: mpsc::Sender<String>
}

impl ClientRouter {

    pub fn new(notifyer: mpsc::Sender<String>) -> Result<Self>
    {
        let context = Context::new();
        let send_recv = context.socket(zmq::DEALER)?;
        let pub_sub = context.socket(zmq::SUB)?;
        send_recv.connect(format!("{}{}", LOCALHOST, PORT1).as_str())?;
        pub_sub.connect(format!("{}{}", LOCALHOST, PORT2).as_str())?;
        Ok(Self {
            context,
            send_recv,
            pub_sub,
            subscriptions: HashMap::<String, Option<Vec<Message>>>::new(),
            notifyer
        })
    }

    pub fn new_with_port(
        notifyer: mpsc::Sender<String>,
        send_recv_port:
        u64, pub_sub_port: u64
    ) -> Result<Self>
    {
        let context = Context::new();
        let send_recv = context.socket(zmq::DEALER)?;
        let pub_sub = context.socket(zmq::SUB)?;
        send_recv.connect(format!("{}{}", LOCALHOST, send_recv_port).as_str())?;
        pub_sub.connect( format!("{}{}", LOCALHOST, pub_sub_port).as_str())?;
        Ok(Self {
            context,
            send_recv,
            pub_sub,
            subscriptions: HashMap::<String,  Option<Vec<Message>>>::new(),
            notifyer
        })
    }

    pub fn send(&self, msg: Vec<Message>) -> Result<()> {
        for m in msg {
            self.send_recv.send(m, 0)?;
        }
        Ok(())
    }

    pub fn receive(&self) -> Result<Vec<Message>> {
        let mut msgs = Vec::<Message>::new();
        while self.send_recv.get_rcvmore()? {
            msgs.push(self.send_recv.recv_msg(0)?);
        }
        Ok(msgs)
    }

    pub fn subscribe(&mut self, topic: &str) {
        self.subscriptions.insert(topic.to_string(), None);
    }

    pub fn get_subscription(&self, topic: &str) -> Result<&Vec<Message>> {
        if let Some(v) = self.subscriptions.get(topic) {
            if let Some(msg) = v {
                return Ok(msg)
            }
        }
        Err(anyhow!("no subscription"))
    }
}


