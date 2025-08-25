pub mod auth;
pub mod error;
pub mod user;

use std::time::Duration;

use bb8::PooledConnection;
use bb8_surrealdb_any::ConnectionManager;
use serde::{Deserialize, Serialize};
use surrealdb::RecordId;

use crate::CONFIG;

pub use error::{Error, Result};

pub type Db<'a> = PooledConnection<'a, ConnectionManager<String>>;

/// Create a new connection pool for the database.
/// # Errors
/// If the database is unreachable, this function will return an error.
pub async fn create_pool() -> crate::Result<bb8::Pool<ConnectionManager<String>>> {
    #[cfg(not(debug_assertions))]
    if &CONFIG.db_url == "memory" {
        return Err(crate::Error::MemoryDatabaseInRelease);
    }

    #[allow(unused_mut)]
    let mut min_idle = 2;
    #[cfg(debug_assertions)]
    if CONFIG.db_url == "memory" {
        min_idle = 1;
    }

    #[allow(unused_mut)]
    let mut max_size = 16;
    #[cfg(debug_assertions)]
    if CONFIG.db_url == "memory" {
        max_size = 1;
    }

    let pool = bb8::Pool::builder()
        .connection_timeout(Duration::from_secs(3))
        .min_idle(min_idle)
        .max_size(max_size)
        .build(ConnectionManager::new(CONFIG.db_url.clone()))
        .await
        .map_err(|e| {
            tracing::error!(?e);
            crate::Error::PoolInitFailed
        })?;

    #[cfg(debug_assertions)]
    {
        let db = pool.get().await.map_err(|e| {
            tracing::error!(?e);
            crate::Error::PoolGetConnectionFailed
        })?;

        tracing::debug!("importing into memory");

        db.use_ns(&CONFIG.db_namespace)
            .use_db(&CONFIG.db_database)
            .await
            .map_err(|e| {
                tracing::error!(?e);
                crate::Error::DbUseNsDbFailed
            })?;

        for entry in glob::glob("sql/*.surql")
            .unwrap()
            .chain(glob::glob("sql/repo/*.surql").unwrap())
            .chain(glob::glob("sql/dev/*.surql").unwrap())
        {
            match entry {
                Err(e) => {
                    tracing::error!(?e);
                }
                Ok(path) => {
                    db.import(path).await.map_err(|e| {
                        tracing::error!(?e);
                        crate::Error::MemoryDatabaseImportFailed
                    })?;
                }
            }
        }
    }

    Ok(pool)
}

#[derive(Serialize, Deserialize)]
pub struct DbRecord<T> {
    #[serde(serialize_with = "serialize_record_id")]
    id: RecordId,
    #[serde(flatten)]
    data: T,
}

fn serialize_record_id<S>(id: &RecordId, serializer: S) -> std::result::Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&id.key().to_string())
}
