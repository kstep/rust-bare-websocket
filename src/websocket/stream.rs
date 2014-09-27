use std::io::{Buffer, Reader, Writer, IoResult, BufferedStream, standard_error};
use std::io::net::tcp::TcpStream;
use std::io;
use openssl::ssl;
use openssl::ssl::{SslStream, SslContext};

pub enum NetworkStream {
    NormalStream(TcpStream),
    SslProtectedStream(SslStream<TcpStream>)
}

impl NetworkStream {
    pub fn connect(ipaddr: &str, port: u16, use_ssl: bool) -> IoResult<NetworkStream> {
        let sock = try!(TcpStream::connect(ipaddr, port));

        if use_ssl {
            let ctx = try!(SslContext::new(ssl::Sslv23).map_err(|e| standard_error(io::OtherIoError)));
            Ok(SslProtectedStream(try!(SslStream::new(&ctx, sock).map_err(|e| standard_error(io::OtherIoError)))))
        } else {
            Ok(NormalStream(sock))
        }
    }
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


