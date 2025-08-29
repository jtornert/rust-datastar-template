use std::fmt::Display;

pub type Result<T> = core::result::Result<T, Error>;
#[derive(Debug)]
pub enum Error {
    Surreal,
    UsernameTaken(String),
    InvalidCredentials(String, Vec<&'static str>),
    CredentialsInvalid,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}
