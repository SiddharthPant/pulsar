use std::{env, net::SocketAddr, time::Duration};

use anyhow::Context;
use askama::Template;
use axum::{
    Router,
    extract::{self, Path, Request, State},
    http::{HeaderName, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tokio::{net::TcpListener, signal};
use tower::ServiceBuilder;
use tower_http::{
    LatencyUnit,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    services::ServeDir,
    timeout::TimeoutLayer,
    trace::{DefaultOnEos, DefaultOnResponse, TraceLayer},
};
use tracing::{error, info_span};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

struct Config {
    database_url: String,
    bind_addr: SocketAddr,
    db_max_connections: u32,
    request_timeout: Duration,
}

impl Config {
    fn from_env() -> anyhow::Result<Self> {
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

enum AppError {
    NotFound(String),
    Internal(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound(message) => (StatusCode::NOT_FOUND, message).into_response(),
            Self::Internal(err) => {
                tracing::error!(error = ?err, "request failed");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err)
    }
}

const REQUEST_ID_HEADER: &str = "x-request-id";

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env()?;
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

    let x_request_id = HeaderName::from_static(REQUEST_ID_HEADER);

    let middleware = ServiceBuilder::new()
        .layer(SetRequestIdLayer::new(
            x_request_id.clone(),
            MakeRequestUuid,
        ))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &Request<_>| {
                    let request_id = request.headers().get(REQUEST_ID_HEADER);
                    match request_id {
                        Some(request_id) => info_span!(
                            "request",
                            request_id = ?request_id,
                            method = %request.method(),
                            uri = %request.uri().path(),
                        ),
                        None => {
                            error!("could not extract request_id");
                            info_span!(
                                "request",
                                method = %request.method(),
                                uri = %request.uri().path(),
                            )
                        }
                    }
                })
                .on_response(DefaultOnResponse::new().latency_unit(LatencyUnit::Micros))
                .on_eos(DefaultOnEos::new().latency_unit(LatencyUnit::Micros)),
        )
        .layer(PropagateRequestIdLayer::new(x_request_id));

    let pool = PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&config.database_url)
        .await
        .context("failed to connect to database")?;

    let state = AppState { pool };

    let app = app()
        .with_state(state)
        .nest_service("/assets", ServeDir::new("assets"))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            config.request_timeout,
        ))
        .layer(middleware);

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

fn app() -> Router<AppState> {
    Router::new()
        .route("/", get(home))
        .route("/greet/{name}", get(greet))
        .route("/users", get(list_users))
        .route("/users/{id}", get(get_user))
        .fallback(not_found)
}

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "nothing to see here")
}

async fn home() -> impl IntoResponse {
    let template = HelloTemplate {};
    HtmlTemplate(template)
}

async fn list_users(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let users = sqlx::query_as!(User, r#"select id, full_name, email from users"#)
        .fetch_all(&state.pool)
        .await
        .context("Failed to retrieve users from database")?;
    Ok(HtmlTemplate(UserListTemplate { users }))
}

async fn get_user(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = sqlx::query_as!(
        User,
        "select id, full_name, email FROM users WHERE id = $1",
        id
    )
    .fetch_optional(&state.pool)
    .await
    .with_context(|| format!("failed to retrieve user {id}"))?
    .ok_or_else(|| AppError::NotFound(format!("user {id} does not exist")))?;

    Ok(HtmlTemplate(UserDetailTemplate { user }))
}

async fn greet(extract::Path(name): extract::Path<String>) -> impl IntoResponse {
    let template = GreetTemplate { name };
    HtmlTemplate(template)
}

#[derive(Template)]
#[template(path = "greet.html")]
struct GreetTemplate {
    name: String,
}

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate;

struct HtmlTemplate<T>(T);

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => AppError::Internal(err.into()).into_response(),
        }
    }
}

#[derive(sqlx::FromRow, Clone)]
struct User {
    id: Uuid,
    full_name: String,
    email: String,
}

#[derive(Template)]
#[template(path = "user_list.html")]
struct UserListTemplate {
    users: Vec<User>,
}

#[derive(Template)]
#[template(path = "user_detail.html")]
struct UserDetailTemplate {
    user: User,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;

    use tower::ServiceExt;

    fn test_app() -> Router {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost/app_db")
            .expect("valid test database URL");

        app().with_state(AppState { pool })
    }

    #[tokio::test]
    async fn greet_returns_html() {
        let response = test_app()
            .oneshot(Request::get("/greet/Foo").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body().collect().await.unwrap().to_bytes();

        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("Hello, Foo!"));
    }

    #[tokio::test]
    async fn unknown_route_returns_not_found() {
        let response = test_app()
            .oneshot(Request::get("/does-not-exist").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
