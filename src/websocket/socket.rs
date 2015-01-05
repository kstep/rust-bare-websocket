use std::io::net::ip::{SocketAddr, Ipv4Addr};
use std::io::net::get_host_addresses;
use std::io::{self, Buffer, Reader, Writer, IoResult, BufferedStream, standard_error};
use std::mem;
use std::collections::BTreeMap;
use url::Url;

#[cfg(test)]
use test::Bencher;
#[cfg(test)]
use serialize::json::ToJson;

use nonce::Nonce;
use message::{WSMessage, WSHeader, WS_MASK, WS_LEN, WS_LEN16, WS_LEN64, WS_OPCONT, WS_OPCODE, WS_FIN};
use stream::NetworkStream;


pub struct WebSocket<S = NetworkStream> {
    stream: Option<BufferedStream<S>>,
    connected: bool,
    pub remote_addr: Option<SocketAddr>,
    pub url: Url,
    use_ssl: bool,
}

impl WebSocket {
    pub fn new(url: Url) -> IoResult<WebSocket> {
        let addr = match try!(url.domain()
            .map(|h| get_host_addresses(h)
                 .map(|v| v.into_iter().find(|&a| {
                     match a {
                         Ipv4Addr(..) => true,
                         _ => false
                     }
                 })))
            .unwrap_or(Err(standard_error(io::InvalidInput)))) {
                Some(a) => a,
                None => return Err(standard_error(io::FileNotFound))
            };

        let use_ssl = url.scheme[] == "wss";

        let port = match url.port() {
            Some(p) => p,
            None if use_ssl => 443,
            _ => 80
        };

        Ok(WebSocket {
            stream: None,
            connected: false,
            remote_addr: Some(SocketAddr{ ip: addr, port: port }),
            url: url,
            use_ssl: use_ssl,
        })
    }

    #[allow(unused_variable)]
    fn try_connect(&mut self) -> IoResult<()> {
        self.stream = Some(BufferedStream::new(try!(self.remote_addr.map(|ref a| NetworkStream::connect(format!("{}:{}", a.ip, a.port)[], self.use_ssl))
                               .unwrap_or_else(|| Err(standard_error(io::InvalidInput))))));
        Ok(())
    }

    fn write_request(&mut self, nonce: &str) -> IoResult<()> {
        let s = match self.stream { Some(ref mut s) => s, None => return Err(standard_error(io::NotConnected)) };

        try!(s.write(format!("GET {} HTTP/1.1\r\n", self.url.serialize_path().unwrap_or("/".to_string())).as_bytes()));
        try!(s.write(format!("Host: {}\r\n", self.url.host().unwrap()).as_bytes()));
        try!(s.write(format!("Origin: {}\r\n", self.url.serialize_no_fragment()).as_bytes()));
        try!(s.write(format!("Sec-WebSocket-Key: {}\r\n", nonce).as_bytes()));

        try!(s.write(b"Upgrade: websocket\r\n"));
        try!(s.write(b"Connection: Upgrade\r\n"));
        try!(s.write(b"Sec-WebSocket-Protocol: char, superchat\r\n"));
        try!(s.write(b"Sec-WebSocket-Version: 13\r\n"));
        try!(s.write(b"\r\n"));

        s.flush()
    }

    fn read_response(&mut self, nonce: &str) -> IoResult<()> {
        let spaces: &[_] = &[' ', '\t', '\r', '\n'];
        let s = match self.stream { Some(ref mut s) => s, None => return Err(standard_error(io::NotConnected)) };
        let status = try!(s.read_line())[].splitn(2, ' ').nth(1).and_then(|s| s.parse::<uint>());

        match status {
            Some(101) => (),
            _ => return Err(standard_error(io::InvalidInput))
        }

        let headers = s.lines().map(|r| r.unwrap_or("\r\n".to_string())) .take_while(|l| l[] != "\r\n")
            .map(|s| s[].splitn(1, ':').map(|s| s.trim_chars(spaces).to_string()).collect::<Vec<String>>())
            .map(|p| (p[0].to_string(), p[1].to_string()))
            .collect::<BTreeMap<String, String>>();

        try!(s.flush());

        let response = headers.find(&"Sec-WebSocket-Accept".to_string());
        match response {
            Some(r) if nonce == r[] => (),
            _ => return Err(standard_error(io::InvalidInput))
        }

        Ok(())
    }

    pub fn connect(&mut self) -> IoResult<()> {
        let mut nonce = Nonce::new();

        try!(self.try_connect());
        try!(self.write_request(nonce[]));

        nonce = nonce.encode();
        try!(self.read_response(nonce[]));

        self.connected = true;

        Ok(())
    }

    fn read_header(&mut self) -> IoResult<WSHeader> {
        // XXX: this is a bug, WSHeader should accept u16
        Ok(WSHeader::from_bits_truncate(try!(self.read_be_u16())))
    }

    fn read_length(&mut self, header: &WSHeader) -> IoResult<uint> {
        let wslen = *header & WS_LEN;
        if wslen == WS_LEN16 { self.read_be_u16().map(|v| v as uint) }
        else if wslen == WS_LEN64 { self.read_be_u64().map(|v| v as uint) }
        else { Ok(wslen.bits() as uint) }
    }

    pub fn read_message(&mut self) -> IoResult<WSMessage> {
        let header = try!(self.read_header());
        let len = try!(self.read_length(&header));

        let data = if header.contains(WS_MASK) {
            WebSocket::unmask_data(try!(self.read_exact(len)), try!(self.read_be_u32()))
        } else {
            try!(self.read_exact(len))
        };

        Ok(WSMessage { header: header, data: data })
    }

    fn unmask_data(data: Vec<u8>, mask: u32) -> Vec<u8> {
        data.iter().enumerate().map(|(i, b)| *b ^ (mask >> ((i % 4) << 3) & 0xff) as u8).collect::<Vec<u8>>()
    }

    // TODO: send_message(&mut self, &WSMessage) -> IoResult<()>

    pub fn iter(&mut self) -> WSMessages {
        WSMessages { sock: self }
    }
}

impl Reader for WebSocket {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<uint> {
        match self.stream {
            Some(ref mut s) => s.read(buf),
            None => Err(standard_error(io::NotConnected))
        }
    }
}

impl Writer for WebSocket {
    fn write(&mut self, buf: &[u8]) -> IoResult<()> {
        match self.stream {
            Some(ref mut s) => s.write(buf),
            None => Err(standard_error(io::NotConnected))
        }
    }

    fn flush(&mut self) -> IoResult<()> {
        match self.stream {
            Some(ref mut s) => s.flush(),
            None => Err(standard_error(io::NotConnected))
        }
    }
}

impl Buffer for WebSocket {
    fn fill_buf<'a>(&'a mut self) -> IoResult<&'a [u8]> {
        match self.stream {
            Some(ref mut s) => s.fill_buf(),
            None => Err(standard_error(io::NotConnected))
        }
    }

    fn consume(&mut self, amt: uint) {
        match self.stream {
            Some(ref mut s) => s.consume(amt),
            None => ()
        }
    }
}

pub struct WSMessages<'a> {
    sock: &'a mut WebSocket
}

pub struct WSDefragMessages<'a> {
    underlying: &'a mut WSMessages<'a>,
    buffer: WSMessage
}

impl<'a> WSMessages<'a> {
    pub fn defrag(&'a mut self) -> WSDefragMessages<'a> {
        WSDefragMessages{ underlying: self, buffer: WSMessage{ header: WSHeader::empty(), data: Vec::new() } }
    }
}

impl<'a> Iterator for WSMessages<'a> {
    type Item = WSMessage;
    fn next(&mut self) -> Option<WSMessage> {
        self.sock.read_message().ok()
    }
}

impl<'a> WSDefragMessages<'a> {
    fn popbuf(&mut self) -> Option<WSMessage> {
        if self.buffer.data.is_empty() {
            None
        } else {
            let mut buf = WSMessage{ header: WSHeader::empty(), data: Vec::new() };
            mem::swap(&mut self.buffer, &mut buf);
            Some(buf)
        }
    }

    fn swapbuf(&mut self, msg: &mut WSMessage) -> () {
        mem::swap(&mut self.buffer, msg);
    }
}

impl<'a> Iterator for WSDefragMessages<'a> {
    type Item = WSMessage;
    fn next(&mut self) -> Option<WSMessage> {
        loop {
            match self.underlying.next() {
                None => return self.popbuf(),
                Some(mut msg) => if msg.header.contains(WS_FIN) {
                    if msg.header & WS_OPCODE == WS_OPCONT {
                        self.buffer.push(msg);
                        return self.popbuf();
                    } else {
                        return Some(msg);
                    }

                } else {
                    if msg.header & WS_OPCODE == WS_OPCONT {
                        self.buffer.push(msg);
                    } else {
                        self.swapbuf(&mut msg);
                        return Some(msg);
                    }
                }
            }
        }
    }
}

#[bench]
#[allow(dead_code)]
fn test_connect(b: &mut Bencher) {
    let url = Url::parse("wss://stream.pushbullet.com/websocket/").unwrap();
    let mut ws = WebSocket::new(url).unwrap();

    match ws.connect() {
        Err(e) => panic!("error: {}", e),
        _ => ()
    }
    let msg = ws.read_message().unwrap();
    println!("received: {} {} {}", msg, msg.to_string(), msg.to_json());
    for msg in ws.iter() {
        println!("{}", msg.to_string());
    }
}
