use std::sync::Arc;
use tokio::io::{self, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

use crate::encoding::Encoding;
use crate::error::ServerError;
use crate::fileops;
use crate::headers::Header;
use crate::request::{Method, Request, Version};

enum Status {
    Ok,
    Created,
    NotFound,
    InternalServerError,
}

impl Status {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Ok => b"200",
            Self::Created => b"201",
            Self::NotFound => b"404",
            Self::InternalServerError => b"500",
        }
    }

    pub fn reason(&self) -> &[u8] {
        match self {
            Self::Ok => b"OK",
            Self::Created => b"Created",
            Self::NotFound => b"Not Found",
            Self::InternalServerError => b"Internal Server Error",
        }
    }
}

struct ResponseLine {
    version: Version,
    status: Status,
}

impl<'a> ResponseLine {
    pub fn new(version: Version, status: Status) -> Self {
        Self { version, status }
    }

    pub async fn write_to<R: AsyncWrite + Unpin>(&self, w: &mut BufWriter<R>) -> io::Result<()> {
        w.write_all(self.version.as_bytes()).await?;
        w.write_u8(b' ').await?;
        w.write_all(self.status.as_bytes()).await?;
        w.write_u8(b' ').await?;
        w.write_all(self.status.reason()).await?;
        w.write_all(b"\r\n").await?;
        Ok(())
    }
}

pub struct Client {
    root_dir: Arc<Option<String>>,
    reader: BufReader<OwnedReadHalf>,
    writer: BufWriter<OwnedWriteHalf>,
}

impl Client {
    const TEXT_PLAIN: (Header, &str) = (Header::ContentType, "text/plain");
    const OCTET_STREAM: (Header, &str) = (Header::ContentType, "application/octet-stream");

    pub fn new(socket: TcpStream, root_dir: Arc<Option<String>>) -> Self {
        let (reader, writer) = socket.into_split();
        Self {
            root_dir,
            reader: BufReader::new(reader),
            writer: BufWriter::new(writer),
        }
    }

    pub async fn run(&mut self) -> Result<(), ServerError> {
        loop {
            let request = Request::from(&mut self.reader).await?;
            if !matches!(
                self.serve(&request).await,
                Ok(_) | Err(ServerError::Disconnected)
            ) {
                self.server_error().await?;
            }
        }
    }

    async fn serve(&mut self, request: &Request) -> Result<(), ServerError> {
        let target = request.target();
        if target == "/" {
            self.ok().await
        } else if target.starts_with("/echo/") {
            let msg = &target[6..];
            self.echo(msg, request).await
        } else if target == "/user-agent" {
            self.user_agent(request).await
        } else if target.starts_with("/files/") {
            let file_name = &target[7..];
            self.file(file_name, request).await
        } else {
            self.not_found().await
        }
    }

    async fn echo(&mut self, msg: &str, request: &Request) -> Result<(), ServerError> {
        let headers = [Self::TEXT_PLAIN];
        match request.accepted_encodings().as_slice() {
            [encoding, ..] => {
                self.ok_with_encoded_body(encoding, &headers, msg.as_bytes())
                    .await
            }
            [] => self.ok_with_body(&headers, msg.as_bytes()).await,
        }
    }

    async fn user_agent(&mut self, request: &Request) -> Result<(), ServerError> {
        let user_agent = request
            .header(&Header::UserAgent)
            .and_then(|v| match v.as_slice() {
                [user_agent] => Some(user_agent.as_bytes()),
                _ => None,
            })
            .ok_or(ServerError::InvalidRequest)?;
        self.ok_with_body(&[Self::TEXT_PLAIN], user_agent).await
    }

    async fn file(&mut self, file_name: &str, request: &Request) -> Result<(), ServerError> {
        let Some(root_dir) = &*self.root_dir else {
            return Err(ServerError::NoRootFolder);
        };
        match request.method() {
            Method::Get => match fileops::read_file(&root_dir, file_name).await {
                Ok(buf) => self.ok_with_body(&[Self::OCTET_STREAM], &buf).await,
                Err(_) => self.not_found().await,
            },
            Method::Post => match request.body() {
                Some(contents) => {
                    fileops::write_file(root_dir, file_name, contents).await?;
                    self.created().await
                }
                None => Err(ServerError::InvalidRequest),
            },
        }
    }

    async fn ok(&mut self) -> Result<(), ServerError> {
        let response_line = ResponseLine::new(Version::Http11, Status::Ok);
        self.send_response(&response_line, &[], None).await
    }

    async fn ok_with_body(
        &mut self,
        headers: &[(Header, &str)],
        body: &[u8],
    ) -> Result<(), ServerError> {
        let response_line = ResponseLine::new(Version::Http11, Status::Ok);
        self.send_response(&response_line, headers, Some(body))
            .await
    }

    async fn ok_with_encoded_body(
        &mut self,
        encoding: &Encoding,
        headers: &[(Header, &str)],
        body: &[u8],
    ) -> Result<(), ServerError> {
        let mut ext_headers = Vec::with_capacity(headers.len() + 1);
        ext_headers.extend_from_slice(headers);
        ext_headers.push((Header::ContentEncoding, encoding.as_str()));
        let response_line = ResponseLine::new(Version::Http11, Status::Ok);
        let enc_body = encoding.encode(body).await?;
        self.send_response(&response_line, &ext_headers, Some(&enc_body))
            .await
    }

    async fn created(&mut self) -> Result<(), ServerError> {
        let response_line = ResponseLine::new(Version::Http11, Status::Created);
        self.send_response(&response_line, &[], None).await
    }

    async fn not_found(&mut self) -> Result<(), ServerError> {
        let response_line = ResponseLine::new(Version::Http11, Status::NotFound);
        self.send_response(&response_line, &[], None).await
    }

    async fn server_error(&mut self) -> Result<(), ServerError> {
        let response_line = ResponseLine::new(Version::Http11, Status::InternalServerError);
        self.send_response(&response_line, &[], None).await
    }

    async fn send_response(
        &mut self,
        response_line: &ResponseLine,
        headers: &[(Header, &str)],
        body: Option<&[u8]>,
    ) -> Result<(), ServerError> {
        response_line.write_to(&mut self.writer).await?;

        for (name, value) in headers {
            self.write_header(name, value).await?;
        }

        match body {
            None => self.writer.write_all(b"\r\n").await?,
            Some(bytes) => {
                let mut buf = itoa::Buffer::new();
                let len = buf.format(bytes.len());
                self.write_header(&Header::ContentLength, len).await?;
                self.writer.write_all(b"\r\n").await?;
                self.writer.write_all(&bytes).await?;
            }
        }

        self.writer.flush().await?;
        Ok(())
    }

    async fn write_header(&mut self, header: &Header, value: &str) -> io::Result<()> {
        self.writer.write_all(header.as_bytes()).await?;
        self.writer.write_all(b": ").await?;
        self.writer.write_all(value.trim().as_bytes()).await?;
        self.writer.write_all(b"\r\n").await?;
        Ok(())
    }
}
