use std::time::Duration;

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
use tokio::{net::TcpListener, signal, time::sleep};
use tower::ServiceBuilder;
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    services::ServeDir,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{error, info_span};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

pub struct AppError(anyhow::Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        eprintln!("Critical Application Error: {:?}", self.0);
        (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self(err)
    }
}

pub trait IntoError {}
impl IntoError for sqlx::Error {}
impl IntoError for askama::Error {}

const REQUEST_ID_HEADER: &str = "x-request-id";

#[derive(Clone)]
struct AppState {
    pool: PgPool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
            TraceLayer::new_for_http().make_span_with(|request: &Request<_>| {
                let request_id = request.headers().get(REQUEST_ID_HEADER);
                match request_id {
                    Some(request_id) => info_span!(
                        "request",
                        request_id = ?request_id,
                        method = %request.method(),
                        uri = %request.uri(),
                    ),
                    None => {
                        error!("could not extract request_id");
                        info_span!(
                            "request",
                            method = %request.method(),
                            uri = %request.uri(),
                        )
                    }
                }
            }),
        )
        .layer(PropagateRequestIdLayer::new(x_request_id));

    let db_url = std::env::var("DATABASE_URL").expect("DB connection URL must be provided");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&db_url)
        .await
        .expect("can't connect to database");

    let state = AppState { pool };

    let app = app()
        .with_state(state)
        .nest_service("/assets", ServeDir::new("assets"))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(10),
        ))
        .layer(middleware);

    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::info!("listening on http://{}/", listener.local_addr().unwrap());
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
        .route("/", get(handler))
        .route("/slow", get(|| sleep(Duration::from_secs(5))))
        .route("/forever", get(std::future::pending::<()>))
        .route("/greet/{name}", get(greet))
        .route("/users", get(list_users))
        .route("/users/{id}", get(get_user))
        .fallback(handler_404)
}

async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "nothing to see here")
}

async fn handler() -> impl IntoResponse {
    let template = HelloTemplate {};
    HtmlTemplate(template)
}

async fn list_users(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let users = sqlx::query_as::<_, User>("select id, name, email from users")
        .fetch_all(&state.pool)
        .await
        .context("Failed to retrieve users directory from database")?;
    Ok(HtmlTemplate(UserListTemplate { users }))
}

async fn get_user(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = sqlx::query_as::<_, User>("select id, full_name, email FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await
        .with_context(|| format!("failed to retrieve user {id}"))?
        .ok_or_else(|| anyhow::anyhow!("user {id} does not exist"))?;

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
    fn into_response(self) -> axum::response::Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to render template. Error: {err}"),
            )
                .into_response(),
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

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use axum::{
//         body::Body,
//         http::{Request, StatusCode},
//     };
//     use http_body_util::BodyExt;
//
//     use tower::ServiceExt;
//
//     #[tokio::test]
//     async fn test_main() {
//         let response = app()
//             .oneshot(Request::get("/greet/Foo").body(Body::empty()).unwrap())
//             .await
//             .unwrap();
//         assert_eq!(response.status(), StatusCode::OK);
//         let body = response.into_body();
//         let bytes = body.collect().await.unwrap().to_bytes();
//         let html = String::from_utf8(bytes.to_vec()).unwrap();
//
//         assert_eq!(html, "<h1>Hello, Foo!</h1>");
//     }
// }
