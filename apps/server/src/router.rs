use std::time::Duration;

use axum::{
    Router,
    extract::Request,
    http::{HeaderName, StatusCode},
    response::IntoResponse,
};
use tower::ServiceBuilder;
use tower_http::{
    LatencyUnit,
    normalize_path::NormalizePath,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    services::ServeDir,
    timeout::TimeoutLayer,
    trace::{DefaultOnEos, DefaultOnResponse, TraceLayer},
};
use tracing::{error, info_span};

use crate::{AppState, auth, home, users};

const REQUEST_ID_HEADER: &str = "x-request-id";

pub fn build_app(state: AppState, request_timeout: Duration) -> NormalizePath<Router> {
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

    let router = Router::new()
        .merge(home::routes())
        .nest("/users", users::routes())
        .nest("/login", auth::routes())
        .fallback(not_found)
        .with_state(state)
        .nest_service("/assets", ServeDir::new("assets"))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            request_timeout,
        ))
        .layer(middleware);
    NormalizePath::trim_trailing_slash(router)
}

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "nothing to see here")
}

#[cfg(test)]
mod tests {
    use super::{REQUEST_ID_HEADER, build_app};
    use crate::AppState;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use sqlx::postgres::PgPoolOptions;
    use std::time::Duration;
    use tower::ServiceExt;
    use tower_http::normalize_path::NormalizePath;
    use uuid::Uuid;

    fn test_app() -> NormalizePath<Router> {
        let pool = PgPoolOptions::new()
            .connect_lazy("postgres://postgres:postgres@localhost/test")
            .expect("test database URL should be valid");

        build_app(AppState { pool }, Duration::from_secs(1))
    }

    fn get(path: &str) -> Request<Body> {
        Request::builder()
            .method("GET")
            .uri(path)
            .body(Body::empty())
            .expect("request should be valid")
    }

    #[tokio::test]
    async fn trailing_slash_is_normalized_before_routing() {
        let canonical = test_app()
            .oneshot(get("/users/not-a-uuid"))
            .await
            .expect("request should succeed");

        let with_slash = test_app()
            .oneshot(get("/users/not-a-uuid/"))
            .await
            .expect("request should succeed");

        assert_eq!(canonical.status(), StatusCode::BAD_REQUEST);
        assert_eq!(with_slash.status(), canonical.status());
    }

    #[tokio::test]
    async fn unknown_route_returns_custom_not_found_response() {
        let response = test_app()
            .oneshot(get("/does-not-exist"))
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = response
            .into_body()
            .collect()
            .await
            .expect("response body should be readable")
            .to_bytes();

        assert_eq!(body.as_ref(), b"nothing to see here");
    }

    #[tokio::test]
    async fn response_contains_generated_request_id() {
        let response = test_app()
            .oneshot(get("/"))
            .await
            .expect("request should succeed");

        let request_id = response
            .headers()
            .get(REQUEST_ID_HEADER)
            .expect("response should contain a request ID")
            .to_str()
            .expect("request ID should be valid text");

        assert!(
            Uuid::parse_str(request_id).is_ok(),
            "generated request ID should be a UUID"
        );
    }

    #[tokio::test]
    async fn existing_request_id_is_preserved() {
        let request = Request::builder()
            .method("GET")
            .uri("/")
            .header(REQUEST_ID_HEADER, "test-request-id")
            .body(Body::empty())
            .expect("request should be valid");

        let response = test_app()
            .oneshot(request)
            .await
            .expect("request should succeed");

        assert_eq!(
            response.headers().get(REQUEST_ID_HEADER).unwrap(),
            "test-request-id"
        );
    }
}
