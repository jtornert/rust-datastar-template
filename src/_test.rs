use std::time::Duration;

use bb8_surrealdb_any::ConnectionManager;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::CONFIG;

type TestError = Box<dyn std::error::Error>;
pub type TestResult = core::result::Result<(), TestError>;

pub mod credentials {
    use crate::repo::auth::Credentials;

    pub fn valid() -> Credentials {
        Credentials::with_username_password("username".into(), "ABcd123+".into())
    }

    pub fn invalid() -> Credentials {
        Credentials::with_username_password("a".into(), "a".into())
    }
}

pub fn setup_test() {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_file(true)
                .with_line_number(true)
                .without_time(),
        )
        .with(EnvFilter::from_default_env())
        .try_init()
        .ok();
}

pub async fn create_test_pool() -> Result<bb8::Pool<ConnectionManager<String>>, TestError> {
    let pool = bb8::Pool::builder()
        .min_idle(1)
        .max_size(1)
        .connection_timeout(Duration::from_millis(100))
        .build(ConnectionManager::new("mem://".into()))
        .await?;

    {
        let db = pool.get().await?;

        db.use_ns(&CONFIG.db_namespace)
            .use_db(&CONFIG.db_database)
            .await?;

        for entry in glob::glob("sql/*.surql")
            .unwrap()
            .chain(glob::glob("sql/repo/*.surql").unwrap())
        {
            match entry {
                Err(e) => {
                    tracing::error!(?e);
                }
                Ok(path) => {
                    db.import(path).await?;
                }
            }
        }
    }

    Ok(pool)
}
