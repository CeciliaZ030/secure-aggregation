use std::ops::Deref;
use anyhow::*;
use p256::ecdsa::signature::Signature;
use p256::elliptic_curve::AffinePoint;
use p256::elliptic_curve::group::GroupEncoding;
use p256::NistP256;
use zmq::Message;


pub enum ClientMessage {
    Handshake(HandshakeReq),
    KeyExchange(KeyExchangeReq),
    InputSharing(InputSharingReq),
}

pub enum ServerMessage {
    // Handshake(HandshakeRes),
    // KeyExchange(KeyExchangeRes),
    //ParamBroadcast(Params),
    InputForward,
}

pub struct HandshakeReq {
    auth_key: p256::ecdsa::VerifyingKey,
    message: String,
}
pub struct KeyExchangeReq {
    ecdh_pubkey: p256::PublicKey,
    enc_ecdh_pubkey:  p256::ecdsa::Signature,
}
pub struct InputSharingReq {
    shares: Vec<u8>,
}

impl From<HandshakeReq> for Vec<Message> {
    fn from(req: HandshakeReq) -> Self {
        let auth_key =  Message::from(convert_affine(req.auth_key.as_affine()));
        let message = Message::from(req.message.as_str());
        vec![auth_key, message]
    }
}
impl TryFrom<Vec<Message>> for HandshakeReq {
    type Error = anyhow::Error;
    fn try_from(msgs: Vec<Message>) -> Result<Self> {
        assert_eq!(msgs.len(), 2);
        let auth_key = p256::ecdsa::VerifyingKey::from_sec1_bytes(msgs[0].deref())?;
        let message = msgs[1].as_str().ok_or(anyhow!("fail"))?;
        Ok(HandshakeReq {
            auth_key,
            message: message.to_string()
        })
    }
}
impl From<KeyExchangeReq> for Vec<Message> {
    fn from(req: KeyExchangeReq) -> Self {
        let ecdh_pubkey =   Message::from(convert_affine(req.ecdh_pubkey.as_affine()));
        let enc_ecdh_pubkey = Message::from(req.enc_ecdh_pubkey.as_bytes());
        vec![ecdh_pubkey, enc_ecdh_pubkey]
    }
}

pub fn convert_affine(point: &AffinePoint<NistP256>) -> Vec<u8> {
    point.to_bytes().as_slice().to_vec()
}
