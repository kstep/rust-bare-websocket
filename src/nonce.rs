use rand::{self, Rng};
use std::ops::Deref;
use rustc_serialize::base64::{self, ToBase64};
use sha1::Sha1;

static WEBSOCKET_GUID: &'static [u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

#[derive(Debug, PartialEq)]
pub struct Nonce(String);

impl Nonce {
    pub fn new() -> Nonce {
        Nonce::generate(&mut rand::thread_rng())
    }

    fn generate<R: Rng>(r: &mut R) -> Nonce {
        let mut nonce = [0u8; 10];
        r.fill_bytes(nonce.as_mut_slice());
        Nonce(nonce.to_base64(base64::STANDARD))
    }

    pub fn encode(self) -> Nonce {
        let mut sha1 = Sha1::new();
        sha1.write_all(self.0.as_bytes()).unwrap();
        sha1.write_all(WEBSOCKET_GUID).unwrap();
        Nonce(sha1.finish().to_base64(base64::STANDARD))
    }
}

impl Deref for Nonce {
    type Target = str;
    fn deref<'a>(&'a self) -> &'a str {
        let Nonce(ref val) = *self;
        &**val
    }
}
