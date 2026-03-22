use tokio::io::{self, AsyncWrite, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

use crate::headers::Header;
use crate::request::{Request, RequestError, Version};

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
    reader: BufReader<OwnedReadHalf>,
    writer: BufWriter<OwnedWriteHalf>,
}

impl Client {
    pub fn new(socket: TcpStream) -> Self {
        let (reader, writer) = socket.into_split();
        Self {
            reader: BufReader::new(reader),
            writer: BufWriter::new(writer),
        }
    }

    pub async fn run(&mut self) -> Result<(), RequestError> {
        let request = Request::from(&mut self.reader).await?;
        self.serve(&request).await?;
        Ok(())
    }

    async fn serve(&mut self, request: &Request) -> io::Result<()> {
        let target = request.request_line.target.as_str();
        if target == "/" {
            let response_line = ResponseLine::new(Version::Http11, Status::Ok);
            self.send_response(&response_line, vec![], None).await
        } else if target.starts_with("/echo/") {
            let msg = &target[6..];
            let response_line = ResponseLine::new(Version::Http11, Status::Ok);
            self.send_response(
                &response_line,
                vec![(Header::ContentType, "text/plain")],
                Some(msg.as_bytes()),
            )
            .await
        } else {
            let response_line = ResponseLine::new(Version::Http11, Status::NotFound);
            self.send_response(&response_line, vec![], None).await
        }
    }

    async fn send_response(
        &mut self,
        response_line: &ResponseLine,
        headers: Vec<(Header, &str)>,
        body: Option<&[u8]>,
    ) -> io::Result<()> {
        response_line.write_to(&mut self.writer).await?;

        for (name, value) in headers {
            self.write_header(name, value).await?;
        }

        match body {
            None => self.writer.write_all(b"\r\n").await?,
            Some(bytes) => {
                let mut buf = itoa::Buffer::new();
                let len = buf.format(bytes.len());
                self.write_header(Header::ContentLength, len).await?;
                self.writer.write_all(b"\r\n").await?;
                self.writer.write_all(&bytes).await?;
            }
        }

        self.writer.flush().await?;
        Ok(())
    }

    async fn write_header(&mut self, header: Header, value: &str) -> io::Result<()> {
        self.writer.write_all(header.as_bytes()).await?;
        self.writer.write_all(b": ").await?;
        self.writer.write_all(value.trim().as_bytes()).await?;
        self.writer.write_all(b"\r\n").await?;
        Ok(())
    }
}
