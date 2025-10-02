use std::fmt::Display;

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

#[macro_export]
macro_rules! log_line {
    ($uuid: expr, $e: expr) => {
        tracing::error!("{} | {} {:?}", $uuid, $e, $e);
    };
    ($uuid: expr) => {
        tracing::error!("{} | Unhandled error", $uuid);
    };
}
