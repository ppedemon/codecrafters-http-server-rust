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
}
