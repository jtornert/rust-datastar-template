use std::fmt::Display;

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    PoolInit,
    PoolGetConnection,
    DbUseNsDb,
    TcpListenerInit,
    ServerStart,
    #[cfg(debug_assertions)]
    MemoryDatabaseImport,
    #[cfg(not(debug_assertions))]
    MemoryDatabaseInRelease,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}
