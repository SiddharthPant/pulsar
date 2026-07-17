use std::{env, net::SocketAddr, time::Duration};

use anyhow::Context;

pub struct Config {
    pub database_url: String,
    pub bind_addr: SocketAddr,
    pub db_max_connections: u32,
    pub request_timeout: Duration,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let database_url = env::var("DATABASE_URL").context("DATABASE_URL must be configured")?;

        let host = env::var("APP_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());

        let port = env::var("APP_PORT")
            .unwrap_or_else(|_| "3000".to_owned())
            .parse::<u16>()
            .context("APP_PORT must be a valid port number")?;

        let bind_addr: SocketAddr = format!("{host}:{port}")
            .parse()
            .context("invalid APP_HOST or APP_PORT")?;

        let db_max_connections = env::var("DB_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "5".to_owned())
            .parse::<u32>()
            .context("DB_MAX_CONNECTIONS must be an integer")?;

        let timeout_seconds = env::var("REQUEST_TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "10".to_owned())
            .parse::<u64>()
            .context("REQUEST_TIMEOUT_SECONDS must be an integer")?;

        Ok(Self {
            database_url,
            bind_addr,
            db_max_connections,
            request_timeout: Duration::from_secs(timeout_seconds),
        })
    }
}
