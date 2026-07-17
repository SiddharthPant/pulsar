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
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    services::ServeDir,
    timeout::TimeoutLayer,
    trace::{DefaultOnEos, DefaultOnResponse, TraceLayer},
};
use tracing::{error, info_span};

use crate::{AppState, home, users};

const REQUEST_ID_HEADER: &str = "x-request-id";

pub fn build_app(state: AppState, request_timeout: Duration) -> Router {
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

    Router::new()
        .merge(home::routes())
        .nest("/users", users::routes())
        .fallback(not_found)
        .with_state(state)
        .nest_service("/assets", ServeDir::new("assets"))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            request_timeout,
        ))
        .layer(middleware)
}

async fn not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "nothing to see here")
}
