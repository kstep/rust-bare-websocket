#![feature(default_type_params)]
#![feature(slicing_syntax)]
#![feature(associated_types)]
#![feature(old_orphan_check)]

extern crate url;
extern crate openssl;
extern crate "rustc-serialize" as rustc_serialize;
extern crate crypto;

#[cfg(test)]
extern crate test;

pub use socket::WebSocket;
pub use message::{WSMessage, WSStatusCode};

pub mod nonce;
pub mod message;
pub mod stream;
pub mod socket;

