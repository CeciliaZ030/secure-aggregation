use anyhow::{anyhow, bail};
use p256::ecdsa::signature::Verifier;
use p256::ecdsa::VerifyingKey;
use router::types::{ClientMessage, ServerReply};
use super::*;

pub struct Worker {
    client_profiles: Arc<RwLock<HashMap<ID, Profile>>>,
    socket: Socket
}

impl Worker {
    pub fn new(
        context: Context,
        client_profiles: Arc<RwLock<HashMap<ID, Profile>>>
    ) -> Result<Self> {
        let socket = context.socket(zmq::PUB)?;
        Ok(Worker {
            client_profiles,
            socket
        })
    }

    pub fn run_task(&self, cur_state: &ServerState) -> Result<()> {
        match cur_state {
            Handshake => self.handshake()?,
            KeyExchange => self.key_exchange()?,
            InputSharing => self.input_sharing()?,
            _ => {}
        }
        Ok(())
    }

    pub fn handshake(&self) -> Result<()> {
        let (id, client_msg) = self._receive()?;
        if let ClientMessage::HS(req) = client_msg {
            let mut profiles = self.client_profiles.write().unwrap();
            if profiles.keys().len() >= MAX_CLIENTS as usize {
                bail!("Client number exceed maximum");
            }
            let profile = Profile {
                auth_key: Some(req.auth_key),
                ecdh_pubkey: None,
                shares: None
            };
            profiles.insert(id, profile);
        }
        bail!("Wrong state");
    }

    pub fn key_exchange(&self) -> Result<()> {
        let (id, client_msg) = self._receive()?;
        if let ClientMessage::KE(req) = client_msg {
            let KeyExchangeReq {
                ecdh_pubkey,
                signature_of_pubkey,
            } = req;
            let mut profiles = self.client_profiles.write().unwrap();
            let auth_key: VerifyingKey = profiles.get(&id)
                .unwrap()
                .auth_key
                .ok_or(anyhow!("We don't have your auth_key"))?;
            auth_key.verify(
                &convert_affine(ecdh_pubkey.as_affine()),
                &signature_of_pubkey
            )?;
        }
        todo!()
    }

    pub fn input_sharing(&self) -> Result<()> {
        todo!()
    }

    pub fn _receive(&self) -> Result<(ID, ClientMessage)> {
        let id : ID = self.socket
            .recv_bytes(zmq::DONTWAIT)?;
        let mut msg = Vec::<Message>::new();
        while self.socket.get_rcvmore()? {
            msg.push(self.socket.recv_msg(zmq::DONTWAIT)?);
        }
        let client_msg = msg.try_into()?;
        Ok((id, client_msg))
    }

    pub fn _send(&self, id: &ID, reply: ServerReply) -> Result<()> {
        self.socket.send(id, zmq::SNDMORE)?;
        let mut msgs: Vec<Message> = reply.into();
        if let Some(last) = msgs.pop() {
            for msg in msgs {
                self.socket.send(msg, zmq::SNDMORE)?;
            }
            self.socket.send(last, zmq::DONTWAIT)?;
        }
        bail!("Empty message")
    }

}











