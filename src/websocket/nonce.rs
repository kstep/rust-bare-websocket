use std::rand::{self, Rng};
use std::ops::Slice;
use serialize::base64::{self, ToBase64};
use crypto::sha1::Sha1;
use crypto::digest::Digest;

static WEBSOCKET_GUID: &'static [u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

#[derive(Show, PartialEq)]
pub struct Nonce(String);

impl Nonce {
    pub fn new() -> Nonce {
        Nonce::generate(&mut rand::thread_rng())
    }

    fn generate(r: &mut Rng) -> Nonce {
        let mut nonce = [0u8; 10];
        r.fill_bytes(nonce.as_mut_slice());
        Nonce(nonce.to_base64(base64::STANDARD))
    }

    pub fn encode(self) -> Nonce {
        let n = match self { Nonce(n) => n };
        let mut sha1 = Sha1::new();
        let mut result = [0u8; 20];
        sha1.input(n.as_bytes());
        sha1.input(WEBSOCKET_GUID);
        sha1.result(result.as_mut_slice());
        Nonce(result.to_base64(base64::STANDARD))
    }
}

impl Slice<uint, str> for Nonce {
    #[inline] fn as_slice_(&self) -> &str {
        match *self { Nonce(ref n) => n[] }
    }

    #[inline] fn slice_from_or_fail(&self, from: &uint) -> &str {
        match *self { Nonce(ref n) => n[*from..] }
    }

    #[inline] fn slice_to_or_fail(&self, to: &uint) -> &str {
        match *self { Nonce(ref n) => n[..*to] }
    }

    #[inline] fn slice_or_fail(&self, from: &uint, to: &uint) -> &str {
        match *self { Nonce(ref n) => n[*from..*to] }
    }
}

impl Str for Nonce {
    fn as_slice<'a>(&'a self) -> &'a str {
        self[]
    }
}
