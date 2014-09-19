#![feature(default_type_params)]

extern crate url;
extern crate openssl;
extern crate serialize;

#[cfg(test)]
extern crate test;

#[cfg(test)]
use test::Bencher;

use url::Url;
use std::io;
use std::io::{Buffer, Reader, Writer, IoResult, BufferedStream, standard_error};
use std::io::net::tcp::TcpStream;
use std::io::net::ip::{SocketAddr, Ipv4Addr};
use std::io::net::get_host_addresses;
use std::fmt::{Show, Formatter, FormatError};
use std::collections::TreeMap;
use serialize::json;
use serialize::json::{Json, ToJson};
use openssl::ssl;
use openssl::ssl::{SslStream, SslContext};

enum NetworkStream {
    NormalStream(TcpStream),
    SslProtectedStream(SslStream<TcpStream>)
}

impl Reader for NetworkStream {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<uint> {
        match *self {
            NormalStream(ref mut s) => s.read(buf),
            SslProtectedStream(ref mut s) => s.read(buf)
        }
    }
}

impl Writer for NetworkStream {
    fn write(&mut self, buf: &[u8]) -> IoResult<()> {
        match *self {
            NormalStream(ref mut s) => s.write(buf),
            SslProtectedStream(ref mut s) => s.write(buf)
        }
    }

    fn flush(&mut self) -> IoResult<()> {
        match *self {
            NormalStream(ref mut s) => s.flush(),
            SslProtectedStream(ref mut s) => s.flush()
        }
    }
}

static WEBSOCKET_GUID: &'static str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

bitflags! {
    #[deriving(Show)] flags WSHeader: u16 {
        static WS_FIN     = 0b1000000000000000,
        static WS_OPCODE  = 0b0000111100000000,
        static WS_MASK    = 0b0000000010000000,
        static WS_LEN     = 0b0000000001111111,

        static WS_LEN16   = 0b0000000001111110,
        static WS_LEN64   = 0b0000000001111111,

        static WS_OPCONT  = 0b0000000000000000,
        static WS_OPTEXT  = 0b0000000100000000,
        static WS_OPBIN   = 0b0000001000000000,
        static WS_OPTERM  = 0b0000100000000000,
        static WS_OPPING  = 0b0000100100000000,
        static WS_OPPONG  = 0b0000101000000000,
    }
}

#[allow(visible_private_types)]
#[allow(dead_code)]
pub struct WebSocket<S = NetworkStream> {
    stream: Option<BufferedStream<S>>,
    handshake_finished: bool,
    pub remote_addr: Option<SocketAddr>,
    pub url: Url,
    use_ssl: bool,
}

#[allow(dead_code)]
impl WebSocket {
    fn new(url: Url) -> IoResult<WebSocket> {
        let addr = match try!(url.domain()
            .map(|h| get_host_addresses(h)
                 .map(|v| v.move_iter().find(|&a| {
                     match a {
                         Ipv4Addr(..) => true,
                         _ => false
                     }
                 })))
            .unwrap_or(Err(standard_error(io::InvalidInput)))) {
                Some(a) => a,
                None => return Err(standard_error(io::FileNotFound))
            };

        let use_ssl = url.scheme.as_slice() == "wss";

        let port = match url.port() {
            Some(p) => p,
            None if use_ssl => 443,
            _ => 80
        };

        Ok(WebSocket {
            stream: None,
            handshake_finished: false,
            remote_addr: Some(SocketAddr{ ip: addr, port: port }),
            url: url,
            use_ssl: use_ssl
        })
    }

    fn connect(&mut self) -> IoResult<()> {
        let s = try!(self.remote_addr.map(|ref a| TcpStream::connect(format!("{}", a.ip).as_slice(), a.port)).unwrap_or_else(|| Err(standard_error(io::InvalidInput))));
        self.stream = Some(BufferedStream::new(
            if self.use_ssl {
                SslProtectedStream(try!(SslStream::new(&try!(SslContext::new(ssl::Sslv23).map_err(|e| standard_error(io::OtherIoError))), s)
                                        .map_err(|e| standard_error(io::OtherIoError))))
            } else {
                NormalStream(s)
            }));
        Ok(())
    }

    fn send_headers(&mut self) -> IoResult<()> {
        let mut s = match self.stream { Some(ref mut s) => s, None => return Err(standard_error(io::NotConnected)) };
        try!(s.write(format!("GET {} HTTP/1.1\r\n", self.url.serialize_path().unwrap_or("/".to_string())).as_bytes()));
        try!(s.write(format!("Host: {}\r\n", self.url.host().unwrap()).as_bytes()));
        try!(s.write("Upgrade: websocket\r\n".as_bytes()));
        try!(s.write("Connection: Upgrade\r\n".as_bytes()));
        try!(s.write(format!("Origin: {}\r\n", self.url.serialize_no_fragment()).as_bytes()));
        try!(s.write("Sec-WebSocket-Protocol: char, superchat\r\n".as_bytes()));
        try!(s.write("Sec-WebSocket-Version: 13\r\n".as_bytes()));
        try!(s.write("Sec-WebSocket-Key: {}\r\n".as_bytes()));
        try!(s.write("\r\n".as_bytes()));
        s.flush()
    }

    fn read_response(&mut self) -> IoResult<()> {
        let spaces: &[_] = &[' ', '\t', '\r', '\n'];
        let mut s = match self.stream { Some(ref mut s) => s, None => return Err(standard_error(io::NotConnected)) };
        let status = try!(s.read_line()).as_slice().splitn(2, ' ').nth(1).and_then(|s| from_str::<uint>(s));

        match status {
            Some(101) => (),
            _ => return Err(standard_error(io::InvalidInput))
        }

        println!("status: {}", status);
        let headers = s.lines().map(|r| r.unwrap_or("\r\n".to_string())) .take_while(|l| l.as_slice() != "\r\n")
            .map(|s| s.as_slice().splitn(1, ':').map(|s| s.trim_chars(spaces).to_string()).collect::<Vec<String>>())
            .map(|p| (p[0].to_string(), p[1].to_string()))
            .collect::<TreeMap<String, String>>();

        try!(s.flush());

        println!("headers: {}", headers);

        if WebSocket::check_nonce("{}", headers.find(&"Sec-WebSocket-Accept".to_string()).unwrap_or(&"".to_string()).as_slice()) {
            Ok(())
        } else {
            Err(standard_error(io::InvalidInput))
        }
    }

    fn check_nonce(nonce: &str, response: &str) -> bool {
        true
    }

    fn handshake(&mut self) -> IoResult<()> {
        try!(self.connect());
        try!(self.send_headers());
        try!(self.read_response());

        self.handshake_finished = true;
        Ok(())
    }

    fn read_header(&mut self) -> IoResult<WSHeader> {
        // XXX: this is a bug, WSHeader should accept u16
        Ok(WSHeader { bits: try!(self.read_be_u16()) as u32 })
    }

    fn read_length(&mut self, header: &WSHeader) -> IoResult<uint> {
        let len = header & WS_LEN;
        println!("hlen: {} & {} = {}", header, WS_LEN, len);
        if len == WS_LEN16 {
            println!("u16 len");
            self.read_be_u16().map(|v| v as uint)
        } else if len == WS_LEN64 {
            println!("u64 len");
            self.read_be_u64().map(|v| v as uint)
        } else {
            Ok(len.bits as uint)
        }
    }

    fn read_message(&mut self) -> IoResult<WSMessage> {
        let header = try!(self.read_header());
        let len = try!(self.read_length(&header));
        let mask = if header.contains(WS_MASK) { Some(try!(self.read_be_u32())) } else { None };
        let data = try!(self.read_exact(len));
        // TODO: unmask data
        Ok(WSMessage { header: header, data: data })
    }

    // TODO: send_message(&mut self, &WSMessage) -> IoResult<()>
    
    fn iter(&mut self) -> WSMessages {
        WSMessages { sock: self }
    }

    fn unmask_data(data: &mut [u8], mask: u32) -> &[u8] {
        data
    }
}

#[deriving(Show)]
struct WSMessage {
    header: WSHeader,
    data: Vec<u8>
}

//impl Show for WSMessage {
    //fn fmt(&self, f: &mut Formatter) -> Result<(), FormatError> {
        //f.write_str("WSMessage(");
        //try!(self.header.fmt(f));
        //try!(self.data.fmt(f));
        //f.write_str(")");
        //Ok(())
    //}
//}

impl WSMessage {
    fn to_string(&self) -> String {
        String::from_utf8_lossy(self.data.as_slice()).into_string()
    }
}

impl ToJson for WSMessage {
    fn to_json(&self) -> Json {
        from_str::<Json>(self.to_string().as_slice()).unwrap()
    }
}

struct WSMessages<'a> {
    sock: &'a mut WebSocket
}

impl<'a> Iterator<WSMessage> for WSMessages<'a> {
    fn next(&mut self) -> Option<WSMessage> {
        self.sock.read_message().ok()
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

#[bench]
fn test_connect(b: &mut Bencher) {
    let url = Url::parse("wss://stream.pushbullet.com/websocket/").unwrap();
    let mut ws = WebSocket::new(url).unwrap();

    match ws.handshake() {
        Err(e) => fail!("error: {}", e),
        _ => ()
    }
    let msg = ws.read_message().unwrap();
    println!("received: {} {} {}", msg, msg.to_string(), msg.to_json());
    for msg in ws.iter() {
        println!("{}", msg.to_string());
    }
}
