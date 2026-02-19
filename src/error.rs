#[derive(Debug)]
pub enum Error {
    Utf8(#[allow(unused)] String),
    Nats(#[allow(unused)] String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}
