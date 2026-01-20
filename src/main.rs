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
