#![feature(slicing_syntax)]

//extern crate "bare-websocket" as websocket;
extern crate websocket;
extern crate url;

use websocket::{WebSocket, WSMessage};
use url::Url;

fn main() {
    let url = Url::parse("ws://echo.websocket.org").unwrap();
    let mut ws = WebSocket::with_options(url, 13, Some(&["chat", "superchat"][]), None);
    ws.connect().unwrap();

    let msg = WSMessage::text("Hello, World!"); //.mask();

    ws.send_message(&msg);

    for m in WSMessage::text("Hello, World!").split(5) {
        println!("msg: {:?} {:?}", m, m.to_string());
        ws.send_message(&m).unwrap();
    }

    let reply = ws.iter().defrag().next();//.unwrap();
    println!("received: {:?} {:?}", reply, reply.as_ref().map(|v| v.to_string()));

    for msg in ws.iter() {
        println!("{:?}", msg.to_string());
    }
}
