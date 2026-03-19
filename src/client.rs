use std::io::ErrorKind::*;

use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("connection closed")]
    Disconnected,
    #[error(transparent)]
    Io(#[from] std::io::Error)
}

pub struct Client {
    socket: TcpStream,
    write_buf: Vec<u8>,
}

impl Client {
    pub fn new(socket: TcpStream) -> Self {
        Self {
            socket,
            write_buf: Vec::with_capacity(1024),
        }
    }

    pub async fn run(&mut self) -> Result<(), ClientError> {
        let _ = self.socket.read_i8().await.map_err(Self::map_err)?;
        self.send_reponse(200).await?;
        Ok(())
    }

    pub async fn send_reponse(&mut self, status: u16) -> Result<(), ClientError> {
        self.write_buf.clear();
        self.write_buf.extend_from_slice(format!("HTTP/1.1 {} OK\r\n", status).as_bytes());
        self.write_buf.extend_from_slice("\r\n".as_bytes());
        self.socket.write_all(&self.write_buf).await.map_err(Self::map_err)?;
        Ok(())
    }

    fn map_err(e: std::io::Error) -> ClientError {
        match e.kind() {
            UnexpectedEof | ConnectionReset | BrokenPipe => ClientError::Disconnected,
            _ => ClientError::Io(e)
        }
    }
}
