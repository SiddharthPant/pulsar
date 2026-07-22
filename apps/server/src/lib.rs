mod auth;
mod dashboard;
mod error;
mod home;
mod page;
mod response;
mod router;
mod users;
use sqlx::postgres::PgPool;

pub use router::build_app;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

impl AppState {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
