use server::CONFIG;
use tokio::net::TcpListener;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().unwrap();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .with_file(true)
                .with_line_number(true),
        )
        .with(EnvFilter::from_default_env())
        .init();

    let pool = server::create_pool().await?;
    let router = server::create_router(pool);
    let listener = TcpListener::bind(&CONFIG.listen_url).await?;

    tracing::info!("listening on http://{}", &CONFIG.listen_url);

    axum::serve(listener, router).await?;

    Ok(())
}
