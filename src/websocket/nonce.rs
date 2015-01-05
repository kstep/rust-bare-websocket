use std::rand::Rng;
use std::rand;
use serialize::base64::ToBase64;
use serialize::base64;
use crypto::sha1::Sha1;
use crypto::digest::Digest;

static WEBSOCKET_GUID: &'static [u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

#[derive(Show, PartialEq)]
pub struct Nonce(String);

impl Nonce {
    pub fn new() -> Nonce {
        Nonce::generate(&mut rand::task_rng())
    }

    fn generate(r: &mut Rng) -> Nonce {
        let mut nonce = [0u8; 10];
        r.fill_bytes(nonce);
        Nonce(nonce.to_base64(base64::STANDARD))
    }

    pub fn encode(self) -> Nonce {
        let n = match self { Nonce(n) => n };
        let mut sha1 = Sha1::new();
        let mut result = [0u8; 20];
        sha1.input(n.as_bytes());
        sha1.input(WEBSOCKET_GUID);
        sha1.result(result);
        Nonce(result.to_base64(base64::STANDARD))
    }
}

impl<'a> Str for Nonce {
    fn as_slice<'a>(&'a self) -> &'a str {
        match *self { Nonce(ref n) => n.as_slice() }
    }
}
