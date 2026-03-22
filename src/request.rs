#![allow(dead_code)]

use std::collections::BTreeMap;
use tokio::io::AsyncRead;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::error::ServerError;
use crate::headers::Header;

pub enum Method {
    Get,
    Post,
}

impl Method {
    pub fn from(s: &str) -> Result<Method, ServerError> {
        match s.trim() {
            "GET" => Ok(Method::Get),
            "POST" => Ok(Method::Post),
            _ => Err(ServerError::InvalidMethod),
        }
    }
}

pub enum Version {
    Http11,
}

impl Version {
    pub fn from(s: &str) -> Result<Version, ServerError> {
        match s.trim() {
            "HTTP/1.1" => Ok(Version::Http11),
            _ => Err(ServerError::InvalidVersion),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Http11 => "HTTP/1.1".as_bytes(),
        }
    }
}

pub struct RequestLine {
    method: Method,
    pub target: String,
    version: Version,
}

impl RequestLine {
    pub fn from(s: &str) -> Result<Self, ServerError> {
        let mut tokens = s.split_ascii_whitespace();
        let method = Method::from(tokens.next().ok_or(ServerError::InvalidRequest)?)?;
        let target = tokens.next().ok_or(ServerError::InvalidRequest)?;
        let version = Version::from(tokens.next().ok_or(ServerError::InvalidRequest)?)?;
        if tokens.next().is_some() {
            Err(ServerError::InvalidRequest)
        } else {
            Ok(Self {
                method,
                target: target.to_string(),
                version,
            })
        }
    }
}

pub struct Headers(pub BTreeMap<Header, Vec<String>>);

impl Headers {
    pub async fn from<R: AsyncRead + Unpin>(r: &mut BufReader<R>) -> Result<Self, ServerError> {
        let mut headers = BTreeMap::new();

        let mut buf = String::with_capacity(512);
        loop {
            buf.clear();
            let n = r.read_line(&mut buf).await?;
            if n == 0 {
                return Err(ServerError::Disconnected);
            }

            let line = buf.trim_end_matches("\r\n");
            if line.is_empty() {
                break;
            }

            let (h, value) = line.split_once(':').ok_or(ServerError::InvalidRequest)?;
            let header = Header::from(h).ok_or(ServerError::InvalidRequest)?;
            let value = value.trim().to_string();
            headers.entry(header).or_insert(Vec::default()).push(value);
        }

        Ok(Self(headers))
    }
}

pub struct Request {
    pub request_line: RequestLine,
    pub headers: Headers,
    body: Option<Vec<u8>>,
}

impl Request {
    pub async fn from<R: AsyncRead + Unpin>(r: &mut BufReader<R>) -> Result<Self, ServerError> {
        let mut buf = String::with_capacity(512);

        let n = r.read_line(&mut buf).await?;
        if n == 0 {
            return Err(ServerError::Disconnected);
        }
        let request_line = RequestLine::from(buf.trim_end_matches("\r\n"))?;
        let headers = Headers::from(r).await?;

        // TODO No body for now
        Ok(Self {
            request_line,
            headers,
            body: None,
        })
    }

    pub fn get_header(&self, h: &Header) -> Option<&Vec<String>> {
      self.headers.0.get(h)
    }
}
