#![allow(clippy::style)]
#![deny(clippy::pedantic)]
#![deny(clippy::nursery)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![cfg_attr(debug_assertions, warn(clippy::pedantic))]
#![cfg_attr(debug_assertions, warn(clippy::nursery))]
#![cfg_attr(debug_assertions, warn(clippy::unwrap_used))]
#![cfg_attr(debug_assertions, warn(clippy::expect_used))]
#![cfg_attr(test, allow(clippy::unwrap_used))]

mod config;
mod resources;

use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use crate::{
    config::CONFIG,
    resources::{AppState, create_router},
};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().unwrap();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .init();
    let router = create_router(AppState::from_default_env().await.unwrap());
    let listener = TcpListener::bind(&CONFIG.listen_url).await.unwrap();
    tracing::info!("{}", CONFIG.listen_url);
    axum::serve(listener, router).await.unwrap();
}
