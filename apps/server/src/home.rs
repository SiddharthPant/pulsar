use crate::{AppState, response::HtmlTemplate};
use askama::Template;
use axum::{Router, response::IntoResponse, routing::get};

pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(home))
}

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate;

async fn home() -> impl IntoResponse {
    let template = HelloTemplate {};
    HtmlTemplate::new(template)
}
