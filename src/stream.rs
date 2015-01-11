use std::io::{self, Reader, Writer, IoResult, standard_error, TcpStream};
use openssl::ssl::{SslMethod, SslStream, SslContext};

pub enum NetworkStream {
    Tcp(TcpStream),
    Ssl(SslStream<TcpStream>)
}

impl NetworkStream {
    pub fn connect(hostname: &str, use_ssl: bool) -> IoResult<NetworkStream> {
        let sock = try!(TcpStream::connect(hostname));

        if use_ssl {
            let ctx = try!(SslContext::new(SslMethod::Sslv23).map_err(|_| standard_error(io::OtherIoError)));
            Ok(NetworkStream::Ssl(try!(SslStream::new(&ctx, sock).map_err(|_| standard_error(io::OtherIoError)))))
        } else {
            Ok(NetworkStream::Tcp(sock))
        }
    }
}

impl Reader for NetworkStream {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        match *self {
            NetworkStream::Tcp(ref mut s) => s.read(buf),
            NetworkStream::Ssl(ref mut s) => s.read(buf)
        }
    }
}

impl Writer for NetworkStream {
    fn write(&mut self, buf: &[u8]) -> IoResult<()> {
        match *self {
            NetworkStream::Tcp(ref mut s) => s.write(buf),
            NetworkStream::Ssl(ref mut s) => s.write(buf)
        }
    }

    fn flush(&mut self) -> IoResult<()> {
        match *self {
            NetworkStream::Tcp(ref mut s) => s.flush(),
            NetworkStream::Ssl(ref mut s) => s.flush()
        }
    }
}


