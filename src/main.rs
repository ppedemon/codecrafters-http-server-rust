use anyhow::Result;
use tokio::net::TcpListener;

use crate::{client::Client, error::ServerError};

mod client;
mod error;
mod headers;
mod request;

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:4221").await?;
    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            let mut client = Client::new(socket);
            if let Err(e) = client.run().await {
                match e {
                    ServerError::Disconnected => {}
                    _ => println!("client error: {}", e),
                }
            }
        });
    }
}
