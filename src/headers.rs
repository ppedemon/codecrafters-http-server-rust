#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum Header {
    Host,
    UserAgent,
    Accept,
    ContentLength,
    ContentType,
}

impl Header {
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Host => b"Host",
            Self::UserAgent => b"User-Agent",
            Self::Accept => b"Accept",
            Self::ContentLength => b"Content-Length",
            Self::ContentType => b"Content-Type",
        }
    }

    pub fn from(s: &str) -> Option<Header> {
        let h = s.as_bytes();
        if h.eq_ignore_ascii_case(b"host") {
            Some(Self::Host)
        } else if h.eq_ignore_ascii_case(b"user-agent") {
            Some(Self::UserAgent)
        } else if h.eq_ignore_ascii_case(b"accept") {
            Some(Self::Accept)
        } else if h.eq_ignore_ascii_case(b"content-length") {
            Some(Self::ContentLength)
        } else if h.eq_ignore_ascii_case(b"content-type") {
            Some(Self::ContentType)
        } else {
            None
        }
    }
}
