use std::sync::LazyLock;

use serde::Deserialize;

#[allow(clippy::unwrap_used)]
pub static CONFIG: LazyLock<Config> = LazyLock::new(|| envy::from_env::<Config>().unwrap());

#[derive(Deserialize)]
pub struct Config {
    pub key: String,
    pub db_url: String,
    pub db_namespace: String,
    pub db_database: String,
    pub db_username: String,
    pub db_password: String,
    pub listen_url: String,
}
