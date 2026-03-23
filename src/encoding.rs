use flate2::{Compression, write::GzEncoder};
use std::io::Write;
use tokio::task;

use crate::error::ServerError;

pub enum Encoding {
    GZip,
}

impl Encoding {
    pub fn as_str(&self) -> &str {
        match self {
            Self::GZip => "gzip",
        }
    }

    pub fn from(s: &str) -> Option<Encoding> {
        if s.eq_ignore_ascii_case("gzip") {
            Some(Self::GZip)
        } else {
            None
        }
    }

    pub async fn encode(&self, buf: &[u8]) -> Result<Vec<u8>, ServerError> {
        match self {
            Self::GZip => {
                let input = buf.to_vec();
                let compressed = task::spawn_blocking(move || {
                    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
                    encoder.write_all(&input)?;
                    encoder.finish()
                })
                .await
                .map_err(|_| ServerError::CompressError)??;
                Ok(compressed)
            }
        }
    }
}
