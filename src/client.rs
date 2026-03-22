use std::sync::Arc;
use tokio::io::{self, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

use crate::error::ServerError;
use crate::fileops;
use crate::headers::Header;
use crate::request::{Request, Version};

enum Status {
    Ok,
    NotFound,
}

impl Status {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Status::Ok => "200".as_bytes(),
            Status::NotFound => "404".as_bytes(),
        }
    }

    pub fn reason(&self) -> &[u8] {
        match self {
            Status::Ok => "OK".as_bytes(),
            Status::NotFound => "Not Found".as_bytes(),
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
    pub fn new(socket: TcpStream, root_dir: Arc<Option<String>>) -> Self {
        let (reader, writer) = socket.into_split();
        Self {
            root_dir,
            reader: BufReader::new(reader),
            writer: BufWriter::new(writer),
        }
    }

    pub async fn run(&mut self) -> Result<(), ServerError> {
        let request = Request::from(&mut self.reader).await?;
        self.serve(&request).await?;
        Ok(())
    }

    async fn serve(&mut self, request: &Request) -> Result<(), ServerError> {
        let target = request.request_line.target.as_str();
        if target == "/" {
            let response_line = ResponseLine::new(Version::Http11, Status::Ok);
            self.send_response(&response_line, &[], None).await
        } else if target.starts_with("/echo/") {
            let msg = &target[6..];
            let response_line = ResponseLine::new(Version::Http11, Status::Ok);
            self.send_response(
                &response_line,
                &[(Header::ContentType, "text/plain")],
                Some(msg.as_bytes()),
            )
            .await
        } else if target == "/user-agent" {
            let user_agent = request
                .get_header(&Header::UserAgent)
                .and_then(|v| match v.as_slice() {
                    [user_agent] => Some(user_agent.as_bytes()),
                    _ => None,
                })
                .ok_or(ServerError::InvalidRequest)?;
            let response_line = ResponseLine::new(Version::Http11, Status::Ok);
            self.send_response(
                &response_line,
                &[(Header::ContentType, "text/plain")],
                Some(user_agent),
            )
            .await
        } else if target.starts_with("/files/") {
            let file_name = &target[7..];
            let Some(root_dir) = &*self.root_dir else {
                return Err(ServerError::NoRootFolder);
            };
            match fileops::read_file(&root_dir, file_name).await {
                Ok(buf) => {
                    let response_line = ResponseLine::new(Version::Http11, Status::Ok);
                    self.send_response(
                        &response_line,
                        &[(Header::ContentType, "application/octet-stream")],
                        Some(&buf),
                    )
                    .await
                }
                Err(_) => {
                    let response_line = ResponseLine::new(Version::Http11, Status::NotFound);
                    self.send_response(&response_line, &[], None).await
                }
            }
        } else {
            let response_line = ResponseLine::new(Version::Http11, Status::NotFound);
            self.send_response(&response_line, &[], None).await
        }
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
