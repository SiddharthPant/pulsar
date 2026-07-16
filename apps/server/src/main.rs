use std::time::Duration;

use askama::Template;
use axum::{
    Router,
    extract::{self, Request},
    http::{HeaderName, StatusCode},
    response::{Html, IntoResponse},
    routing::get,
};
use sqlx::postgres::PgPoolOptions;
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

const REQUEST_ID_HEADER: &str = "x-request-id";

#[tokio::main]
async fn main() {
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
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&db_url)
        .await
        .expect("can't connect to database");

    let app = app()
        .with_state(db_pool)
        .nest_service("/assets", ServeDir::new("assets"))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            Duration::from_secs(10),
        ))
        .layer(middleware);

    let listener = TcpListener::bind("127.0.0.1:3000").await.unwrap();
    tracing::info!("listening on http://{}/", listener.local_addr().unwrap());
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
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

// Utility function to map any error to 500 Internal server error
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

fn app() -> Router {
    Router::new()
        .route("/", get(handler))
        .route("/slow", get(|| sleep(Duration::from_secs(5))))
        .route("/forever", get(std::future::pending::<()>))
        .route("/greet/{name}", get(greet))
        .fallback(handler_404)
}

async fn handler_404() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "nothing to see here")
}

async fn handler(State(pool): State<PgPool>) -> impl IntoResponse {
    let template = HelloTemplate;
    sqlx::query_scalar("select 'hello world from pg'")
        .fetch_one(&pool)
        .await
        .map_err(internal_error);
    HtmlTemplate(template)
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;

    use tower::ServiceExt;

    #[tokio::test]
    async fn test_main() {
        let response = app()
            .oneshot(Request::get("/greet/Foo").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body();
        let bytes = body.collect().await.unwrap().to_bytes();
        let html = String::from_utf8(bytes.to_vec()).unwrap();

        assert_eq!(html, "<h1>Hello, Foo!</h1>");
    }
}
