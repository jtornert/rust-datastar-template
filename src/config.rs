use std::sync::LazyLock;

use serde::Deserialize;

pub static CONFIG: LazyLock<Config> = LazyLock::new(|| envy::from_env::<Config>().unwrap());

#[derive(Deserialize)]
pub struct Config {
    pub listen_url: String,
    pub nats_servers: Vec<String>,
}
