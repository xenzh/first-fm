use std::net::{TcpStream, SocketAddr};
use std::io::{Result, Error};

use r2d2::ManageConnection;

// ----------------------------------------------------------------

#[derive(Debug)]
pub struct TcpStreamManager {
    addr: SocketAddr,
}

impl TcpStreamManager {
    pub fn new(addr: SocketAddr) -> TcpStreamManager {
        TcpStreamManager { addr }
    }
}

impl ManageConnection for TcpStreamManager {
    type Connection = TcpStream;
    type Error = Error;

    fn connect(&self) -> Result<Self::Connection> {
        TcpStream::connect(self.addr)
    }

    fn is_valid(&self, _: &mut Self::Connection) -> Result<()> {
        Ok(())
    }

    fn has_broken(&self, _: &mut Self::Connection) -> bool {
        false
    }
}
