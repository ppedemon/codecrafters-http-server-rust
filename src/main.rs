use anyhow::Result;
use tokio::net::TcpListener;

use crate::client::{Client, ClientError};

mod client;

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:4221").await?;
    loop {
        let (socket, _) = listener.accept().await?;
        tokio::spawn(async move {
            let mut client = Client::new(socket);
            if let Err(e) = client.run().await {
                match e {
                    ClientError::Disconnected => {},
                    _ => println!("client error: {}", e),
                }
            }
        });
    }
}
