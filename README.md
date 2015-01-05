rust-websocket
==============

WebSocket implementation in Rust (@rust-lang)

Example:

```rust
// Initialization
let url = Url::parse("ws://echo.websocket.org").unwrap();
let mut ws = WebSocket::new(url);
ws.connect().unwrap(); // <-- you can pass configured WebSocket somewhere before connecting

let msg = WSMessage::text("Hello, World!"); //.mask(); // <-- optionally turn on automasking
// All masking/unmasking is done transparently, you will never even know about it!

// You can compose fragmented messages as well:
let msg1 = WSMessage::text("Hello").first(); // <-- first fragment
let msg2 = WSMessage::text(", ").more(); // <-- continue fragment
let msg3 = WSMessage::text("world!").last(); // <-- last fragment

// Send all fragments
ws.send_message(&msg1).unwrap();
ws.send_message(&msg2).unwrap();
ws.send_message(&msg3).unwrap();

// Usage of .defrag()menting iterator below (you can get each message fragment by not using it)
let reply = ws.iter().defrag().next().unwrap();
println!("received: {} {}", reply, reply.to_string());

// You can get single message as well:
//let msg = ws.read_message().unwrap();

// Simple messages iterator, to handle defragmentation, append .defrag() after .iter()
for msg in ws.iter() {
    println!("{}", msg.to_string());
}
```
