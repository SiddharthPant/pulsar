mod config;
mod error;
mod home;
mod response;
mod router;
mod users;
use std::{env, time::Duration};

use anyhow::Context;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::{net::TcpListener, signal};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = config::Config::from_env()?;
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!(
                    "{}=debug,tower_http=debug,axum=trace",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    let pool = PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&config.database_url)
        .await
        .context("failed to connect to database")?;

    let state = AppState { pool };

    let app = router::build_app(state, config.request_timeout);

    let listener = TcpListener::bind(config.bind_addr)
        .await
        .context("failed to bind HTTP listener")?;

    let address = listener
        .local_addr()
        .context("failed to read listener address")?;

    tracing::info!(%address, "server listening");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
