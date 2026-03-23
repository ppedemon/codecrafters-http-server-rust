use std::sync::Arc;

use anyhow::Result;
use clap::Parser;
use tokio::net::TcpListener;

use crate::{client::Client, error::ServerError};

mod client;
mod encoding;
mod error;
mod fileops;
mod headers;
mod request;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    #[arg(long)]
    directory: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let directory = Arc::new(args.directory);

    let listener = TcpListener::bind("127.0.0.1:4221").await?;
    loop {
        let (socket, _) = listener.accept().await?;
        let root_dir = Arc::clone(&directory);

        tokio::spawn(async move {
            let mut client = Client::new(socket, root_dir);
            if let Err(e) = client.run().await {
                if !matches!(e, ServerError::Disconnected) {
                    eprintln!("client error: {}", e);
                }
            }
        });
    }
}
