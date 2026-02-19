#![allow(clippy::style)]
#![deny(
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used
)]
#![cfg_attr(
    debug_assertions,
    warn(
        clippy::pedantic,
        clippy::nursery,
        clippy::unwrap_used,
        clippy::expect_used
    )
)]
#![cfg_attr(test, allow(clippy::unwrap_used))]

mod config;
mod error;
mod repo;
mod resources;

use std::net::SocketAddr;

use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use crate::{
    config::CONFIG,
    error::Error,
    resources::{AppState, create_router},
};

#[allow(clippy::unwrap_used)]
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
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
