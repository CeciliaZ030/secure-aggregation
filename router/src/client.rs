use std::collections::{HashMap, HashSet};
use std::fmt::format;
use std::convert::*;
use std::ops::Deref;
use std::sync::{Arc, mpsc, Mutex};
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

const LOCALHOST: &'static str = "tcp://localhost:";
const PORT1: u64 = 7777;
const PORT2: u64 = 7676;

async fn mainn() -> Result<()> {
    let mut c = ClientRouter::new()?;
    c.subscribe("sss")?;
    c.run_subscriber()?;
    let r = c.wait_on_subscription("sss").await;
    Ok(())
}


pub struct ClientRouter {
    context: Context,
    send_recv: Socket,
    pub_sub_handle: Option<JoinHandle<Result<()>>>,
    subscription: Arc<Mutex<HashSet<String>>>,
    inbox: Arc<Mutex<HashMap<String, Vec<Message>>>>,
}

impl ClientRouter {

    pub fn new() -> Result<Self> {

        let context = Context::new();
        let send_recv = context.socket(zmq::DEALER)?;
        let pub_sub = context.socket(zmq::SUB)?;
        send_recv.connect(format!("{}{}", LOCALHOST, PORT1).as_str())?;
        pub_sub.connect(format!("{}{}", LOCALHOST, PORT2).as_str())?;

        let set = HashSet::<String>::new();
        let map = HashMap::<String, Vec<Message>>::new();

        Ok(Self {
            context,
            send_recv,
            pub_sub_handle: None,
            subscription: Arc::new(Mutex::new(set)),
            inbox: Arc::new(Mutex::new(map)),
        })
    }

    // pub fn new_with_port(
    //     send_recv_port:
    //     u64, pub_sub_port: u64
    // ) -> Result<Self> {
    //     let context = Context::new();
    //     let send_recv = context.socket(zmq::DEALER)?;
    //     let pub_sub = context.socket(zmq::SUB)?;
    //     send_recv.connect(format!("{}{}", LOCALHOST, send_recv_port).as_str())?;
    //     pub_sub.connect( format!("{}{}", LOCALHOST, pub_sub_port).as_str())?;
    //     let map = HashMap::<String, Option<Vec<Message>>>::new();
    //     Ok(Self {
    //         context,
    //         send_recv,
    //         pub_sub,
    //         inbox: Arc::new(Mutex::new(map)),
    //     })
    // }


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

    pub fn run_subscriber(&mut self) -> Result<()> {
        let subscription = self.subscription.clone();
        let inbox = self.inbox.clone();
        let pub_sub = self.context.socket(zmq::SUB)?;
        pub_sub.connect(format!("{}{}", LOCALHOST, PORT2).as_str())?;
        let handle = spawn(move || -> Result<()> {
            loop {
                if pub_sub.get_rcvmore()? {
                    let mut msgs = pub_sub.recv_multipart(0)?;
                    let topic = String::from_utf8(msgs.remove(0))?;
                    if subscription.lock().unwrap().contains(topic.as_str()) {
                        let mut inbox_guard = inbox.lock().unwrap();
                        if inbox_guard.contains_key(topic.as_str()) {
                            continue
                        } else {
                            inbox_guard.insert(
                                topic,
                                msgs.iter()
                                    .map(|msg| Message::from(msg.as_slice()))
                                    .collect()
                            );
                        }
                    }
                } else {
                    continue
                }
            }
        });
        self.pub_sub_handle = Option::from(handle);
        Ok(())
    }

    pub fn subscribe(&self, topic: &str) -> Result<()> {
        let guard = self.subscription.clone();
        guard.lock()
            .unwrap()
            .insert(topic.to_string());
        Ok(())
    }

    pub fn try_take_subscription(&self, topic: &str) -> Result<Vec<Message>> {
        if let Some(msg) = self.inbox
            .lock()
            .unwrap()
            .remove(topic)
        {
           return Ok(msg)
        }
        Err(anyhow!("no subscribed topics arrived"))
    }

    async fn wait_on_subscription(&self, topic: &str) -> Result<Vec<Message>> {
        while !self.inbox
            .lock()
            .unwrap()
            .contains_key(topic)
        {
            continue
        }
        self.inbox.lock().unwrap()
            .remove(topic)
            .ok_or(anyhow!("failed getting subscription"))
    }

}


