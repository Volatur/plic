use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};

#[derive(Debug)]
pub struct Server {
    pub listener: TcpListener,
}

#[derive(Debug)]
pub struct Connection {
    pub stream: TcpStream
}

impl Server {
    pub fn new(addr: String) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        Ok(Self { listener })
    }

    pub fn accept(&self) -> Result<Option<(Connection, SocketAddr)>, std::io::Error> {
        match self.listener.accept() {
            Ok((stream, addr)) => Ok(Some((Connection::new(stream), addr))),
            Err(ref err) if err.kind() == ErrorKind::WouldBlock => Ok(None),
            Err(err) => Err(err)
        }
    }
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        stream.set_nonblocking(true).unwrap();
        Self { stream }
    }

    pub fn send(&mut self, message: String) -> Result<(), std::io::Error> {
        self.stream.write(message.as_bytes())?;
        self.stream.flush()?;
        Ok(())
    }

    pub fn recv(&mut self) -> Result<Option<String>, std::io::Error> {
        let mut message = Vec::new();
        loop {
            let mut buff = [0u8; 1024];
            match self.stream.read(&mut buff) {
                Ok(1..) => message.extend_from_slice(&buff),
                Ok(0) => break,
                Err(ref err) if err.kind() == ErrorKind::WouldBlock => break,
                Err(err) => return Err(err)
            }
        }
        if message.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(String::from_utf8(message.to_vec()).unwrap().trim_end_matches('\0').trim_end().parse().unwrap()))
        }
    }
    
    pub fn close(&mut self) -> Result<(), std::io::Error> {
        self.stream.shutdown(Shutdown::Both)
    }
}
