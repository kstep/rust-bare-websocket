bare-websocket
================

WebSocket implementation in Rust (@rust-lang) according to [RFC6455][]

Unlike other [websocket][] library, this library is focused on more low level
details of WebSocket protocol: you can inspect each frame individually, as well
as frame header. You can prepare each frame, along with all header details,
bit-by-bit. It also supports [masking][], custom (service-specific) opcodes and
reserved bits.

On the other hand it implements WebSocket client side only (but message
structs are universal and can be used to implement both client and server
parts).

If you don't need to mess with all the protocol scary details, want more high
level interface and ready WebSocket server implementation, use [websocket][].

[RFC6455]: https://tools.ietf.org/html/rfc6455
[websocket]: http://cyderize.github.io/rust-websocket/
[masking]: https://tools.ietf.org/html/rfc6455#section-5.3

To use, add dependency to you Cargo.toml (you will also need url crate):

```
[dependencies]
url = "*"
bare-websocket = "*"
```

And then in your code:

```rust
extern crate url;
extern crate "bare-websocket" as websocket;

use websocket::{WebSocket, WSMessage};
use url::Url;
```

Example code:

```rust
// Initialization
let url = Url::parse("ws://echo.websocket.org").unwrap(); // <-- also supports SSL, just use "wss://" schema
let mut ws = WebSocket::new(url);
ws.connect().unwrap(); // <-- you can pass configured WebSocket somewhere before connecting

let msg = WSMessage::text("Hello, World!"); //.mask(); // <-- optionally turn on automasking
// All masking/unmasking is done transparently, you will never even know about it!

// You can compose fragmented messages as well:
let msg1 = WSMessage::text("Hello").first(); // <-- first fragment
let msg2 = WSMessage::text(", ").more(); // <-- continue fragment
let msg3 = WSMessage::text("world!").last(); // <-- last fragment

// Or easier: split message by size:
for m in WSMessage::text("Hello, world!").split(5) { // <-- WSMessage iterator
    ws.send_message(&m).unwrap();
}

// Usage of .defrag()menting iterator below (you can get each message fragment by not using it)
let reply = ws.iter().defrag().next().unwrap();
println!("received: {} {}", reply, reply.to_string());

// You can get single message as well:
//let msg = ws.read_message().unwrap();

// Simple messages iterator, to handle defragmentation, append .defrag() after .iter()
for msg in ws.iter() {
    println!("{}", msg.to_string());
}

// To take full bitwise control of opcode field, use `.ext()` method
let msg = WSMessage::ext(0b1011, b"bare metal message"); // <-- this is an extension control opcode

// There are also a lot of `.is_???()` methods to inspect:
println!("{}", msg.is_ext(0b1011)); // is it given extended opcode?
println!("{}", msg.is_control()); // is it control or data opcode?
println!("{}", msg.is_text()); // is it a text message?
println!("{}", msg.is_binary()); // is it a binary message?
// Also exist: .is_ping(), .is_pong(), .is_close(), .is_cont()

```

That's pretty much all of it, actually.
