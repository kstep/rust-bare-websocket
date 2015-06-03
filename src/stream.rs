use openssl::ssl::{SslMethod, SslStream, SslContext};
use std::net::TcpStream;
use std::io::{Write, Read, self};

pub enum NetworkStream {
    Tcp(TcpStream),
    Ssl(SslStream<TcpStream>)
}

impl NetworkStream {
    pub fn connect(hostname: &str, use_ssl: bool) -> io::Result<NetworkStream> {
        let sock = try!(TcpStream::connect(hostname));

        if use_ssl {
            let ctx = try!(SslContext::new(SslMethod::Sslv23).map_err(|_| io::Error::new(io::ErrorKind::Other, "ssl context creation error", None)));
            Ok(NetworkStream::Ssl(try!(SslStream::new(&ctx, sock).map_err(|_| io::Error::new(io::ErrorKind::Other, "ssl connection error", None)))))
        } else {
            Ok(NetworkStream::Tcp(sock))
        }
    }
}

impl Read for NetworkStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match *self {
            NetworkStream::Tcp(ref mut s) => s.read(buf),
            NetworkStream::Ssl(ref mut s) => s.read(buf)
        }
    }
}

impl Write for NetworkStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match *self {
            NetworkStream::Tcp(ref mut s) => s.write(buf),
            NetworkStream::Ssl(ref mut s) => s.write(buf)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match *self {
            NetworkStream::Tcp(ref mut s) => s.flush(),
            NetworkStream::Ssl(ref mut s) => s.flush()
        }
    }
}


