use crate::utils::socket::server::Connection;
use std::net::TcpStream;

#[derive(Debug)]
pub struct Client {
    pub connection: Connection
}

impl Client {
    pub fn new(addr: String) -> Result<Self, std::io::Error> {
        Ok(Self { connection: Connection::new(TcpStream::connect(addr)?) })
    }

    pub fn send(&mut self, message: String) -> Result<(), std::io::Error> {
        self.connection.send(message)
    }

    pub fn recv(&mut self) -> Result<Option<String>, std::io::Error> {
        self.connection.recv()
    }
    
    pub fn close(&mut self) -> Result<(), std::io::Error> {
        self.connection.close()
    }
}