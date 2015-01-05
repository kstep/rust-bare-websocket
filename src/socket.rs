use std::io::{self, Buffer, Reader, Writer, IoResult, BufferedStream, standard_error};
use std::mem;
use std::collections::BTreeMap;
use std::rand::{thread_rng, Rng};
use std::{u64, u16};
use std::num::{Int, FromPrimitive, ToPrimitive};
use std::slice::SliceConcatExt;
use url::Url;

#[cfg(test)]
use test::Bencher;

use nonce::Nonce;
use message::{WSMessage, WSHeader, WS_MASK, WS_LEN, WS_LEN16, WS_LEN64, WS_OPTERM};
use stream::NetworkStream;


pub struct WebSocket<S = NetworkStream> {
    stream: Option<BufferedStream<S>>,
    pub url: Url,
    hostname: String,
    use_ssl: bool,
    version: uint,
    extensions: Option<Vec<String>>,
    protocols: Option<Vec<String>>
}

impl WebSocket {
    pub fn with_options(url: Url, version: uint, protocols: Option<&[&str]>, extensions: Option<&[&str]>) -> WebSocket {
        let use_ssl = url.scheme[] == "wss";

        let port = match url.port() {
            Some(p) => p,
            None if use_ssl => 443,
            _ => 80
        };

        WebSocket {
            stream: None,
            hostname: format!("{}:{}", url.serialize_host().unwrap(), port),
            url: url,
            use_ssl: use_ssl,
            version: version,
            extensions: extensions.map(|v| v.iter().map(|v| v.to_string()).collect()),
            protocols: protocols.map(|v| v.iter().map(|v| v.to_string()).collect())
        }
    }

    #[inline] pub fn new(url: Url) -> WebSocket {
        WebSocket::with_options(url, 1, None, None)
    }

    fn try_connect(&mut self) -> IoResult<()> {
        self.stream = Some(BufferedStream::new(try!(NetworkStream::connect(self.hostname[], self.use_ssl))));
        Ok(())
    }

    fn write_request(&mut self, nonce: &str) -> IoResult<()> {
        let s = match self.stream { Some(ref mut s) => s, None => return Err(standard_error(io::IoErrorKind::NotConnected)) };

        try!(s.write(format!("GET {} HTTP/1.1\r\n", self.url.serialize_path().unwrap_or("/".to_string())).as_bytes()));
        try!(s.write(format!("Host: {}\r\n", self.url.host().unwrap()).as_bytes()));
        try!(s.write(format!("Origin: {}\r\n", self.url.serialize_no_fragment()).as_bytes()));
        try!(s.write(format!("Sec-WebSocket-Key: {}\r\n", nonce).as_bytes()));

        try!(s.write(b"Upgrade: websocket\r\n"));
        try!(s.write(b"Connection: Upgrade\r\n"));
        try!(s.write(format!("Sec-WebSocket-Version: {}\r\n", self.version).as_bytes()));
        if let Some(ref protos) = self.protocols {
            try!(s.write(format!("Sec-WebSocket-Protocol: {}\r\n", protos.connect(", ")).as_bytes()));
        }
        if let Some(ref exts) = self.extensions {
            try!(s.write(format!("Sec-WebSocket-Extensions: {}\r\n", exts.connect(", ")).as_bytes()));
        }
        try!(s.write(b"\r\n"));

        s.flush()
    }

    fn read_response(&mut self, nonce: &str) -> IoResult<()> {
        let spaces: &[_] = &[' ', '\t', '\r', '\n'];
        let s = match self.stream { Some(ref mut s) => s, None => return Err(standard_error(io::IoErrorKind::NotConnected)) };
        let status = try!(s.read_line())[].splitn(2, ' ').nth(1).and_then(|s| s.parse::<uint>());

        match status {
            Some(101) => (),
            _ => return Err(standard_error(io::InvalidInput))
        }

        let headers = s.lines().map(|r| r.unwrap_or("\r\n".to_string())) .take_while(|l| l[] != "\r\n")
            .map(|s| s[].splitn(1, ':').map(|s| s.trim_matches(spaces).to_string()).collect::<Vec<String>>())
            .map(|p| (p[0].to_string(), p[1].to_string()))
            .collect::<BTreeMap<String, String>>();

        try!(s.flush());

        let response = headers.get("Sec-WebSocket-Accept");
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
        let mut len = try!(self.read_length(&header));

        let mask = if header.contains(WS_MASK) {
            Some(try!(self.read_be_u32()))
        } else {
            None
        };

        // If this is the terminating frame (close command),
        // first two bytes of data MUST BE u16 status code
        let mut status = if header.contains(WS_OPTERM) {
            // compensate length of status code
            len = len - 2;
            Some(try!(self.read_be_u16()))
        } else {
            None
        };

        let mut data = try!(self.read_exact(len));

        // If we have mask, decrypt data
        if let Some(mut m) = mask {
            // decrypt status if present
            if let Some(s) = status {
                status = Some(s ^ (m & 0xffff) as u16);
                // compensate the usage of two mask bytes
                m = m.rotate_right(16);
            }
            data = WebSocket::mask_data(data[], m);
        }

        Ok(WSMessage { header: header, data: data, status: status.and_then(FromPrimitive::from_u16) })
    }

    fn mask_data(data: &[u8], mask: u32) -> Vec<u8> {
        data.iter().enumerate().map(|(i, b)| *b ^ (mask >> ((i % 4) << 3) & 0xff) as u8).collect::<Vec<u8>>()
    }

    pub fn send_message(&mut self, msg: &WSMessage) -> IoResult<()> {
        let mut len = msg.data.len();
        let mut hdr = msg.header - WS_LEN;

        // If we have status set, the data length is increased by status size
        if msg.status.is_some() {
            len = len + 2;
        }

        // Encode and send length along with header
        if len < u16::MAX as uint {
            hdr = hdr | WSHeader::from_bits_truncate(len as u16);
            try!(self.write_be_u16(hdr.bits()));

        } else if len < u64::MAX as uint {
            hdr = hdr | WS_LEN16;
            try!(self.write_be_u16(hdr.bits()));
            try!(self.write_be_u16(len as u16));

        } else {
            hdr = hdr | WS_LEN64;
            try!(self.write_be_u16(hdr.bits()));
            try!(self.write_be_u64(len as u64));
        }

        // If user required masking, encrypt all data
        if hdr.contains(WS_MASK) {
            // Generate and send random mask
            let mut mask = thread_rng().gen::<u32>();
            try!(self.write_be_u32(mask));

            // Encrypt status code if present
            if let Some(status) = msg.status {
                try!(self.write_be_u16(status.to_u16().unwrap() ^ (mask & 0xffff) as u16));
                // compensate for mask already used for status encryption
                mask = mask.rotate_right(16);
            }

            try!(self.write(WebSocket::mask_data(msg.data[], mask)[]));
        } else {
            // Send status code if present
            if let Some(status) = msg.status {
                try!(self.write_be_u16(status.to_u16().unwrap()));
            }
            try!(self.write(msg.data[]));
        }

        self.flush()
    }

    pub fn iter(&mut self) -> WSMessages {
        WSMessages { sock: self }
    }
}

impl Reader for WebSocket {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<uint> {
        match self.stream {
            Some(ref mut s) => s.read(buf),
            None => Err(standard_error(io::IoErrorKind::NotConnected))
        }
    }
}

impl Writer for WebSocket {
    fn write(&mut self, buf: &[u8]) -> IoResult<()> {
        match self.stream {
            Some(ref mut s) => s.write(buf),
            None => Err(standard_error(io::IoErrorKind::NotConnected))
        }
    }

    fn flush(&mut self) -> IoResult<()> {
        match self.stream {
            Some(ref mut s) => s.flush(),
            None => Err(standard_error(io::IoErrorKind::NotConnected))
        }
    }
}

impl Buffer for WebSocket {
    fn fill_buf<'a>(&'a mut self) -> IoResult<&'a [u8]> {
        match self.stream {
            Some(ref mut s) => s.fill_buf(),
            None => Err(standard_error(io::IoErrorKind::NotConnected))
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
        WSDefragMessages{ underlying: self, buffer: WSMessage{ header: WSHeader::empty(), data: Vec::new(), status: None } }
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
            let mut buf = WSMessage{ header: WSHeader::empty(), data: Vec::new(), status: None };
            mem::swap(&mut self.buffer, &mut buf);
            Some(buf)
        }
    }

    fn swapbuf(&mut self, msg: &mut WSMessage) {
        mem::swap(&mut self.buffer, msg);
    }
}

impl<'a> Iterator for WSDefragMessages<'a> {
    type Item = WSMessage;
    fn next(&mut self) -> Option<WSMessage> {
        loop {
            match self.underlying.next() {
                None => return self.popbuf(),
                Some(mut msg) => {
                    if msg.is_whole() {
                        return Some(msg);
                    } else if msg.is_first() {
                        self.swapbuf(&mut msg);
                    } else if msg.is_more() {
                        self.buffer.push(msg);
                    } else if msg.is_last() {
                        self.buffer.push(msg);
                        return self.popbuf().map(|v| v.last());
                    }
                }
            }
        }
    }
}

