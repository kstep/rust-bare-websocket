#![crate_name = "websocket"]

#![comment = "WebSocket client"]
#![license = "MIT/ASL2"]
#![crate_type = "dylib"]
#![crate_type = "rlib"]

#![feature(default_type_params)]
#![feature(slicing_syntax)]
#![feature(associated_types)]
#![feature(old_orphan_check)]

extern crate url;
extern crate openssl;
extern crate serialize;
extern crate crypto;

#[cfg(test)]
extern crate test;

pub mod nonce;
pub mod message;
pub mod stream;
pub mod socket;

