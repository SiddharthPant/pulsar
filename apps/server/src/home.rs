use crate::{AppState, error::AppError, page::PageContext, response::HtmlTemplate};
use askama::Template;
use axum::{Router, response::IntoResponse, routing::get};
use tower_sessions::Session;

pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(home))
}

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate {
    page: PageContext,
}

async fn home(session: Session) -> Result<impl IntoResponse, AppError> {
    let page = PageContext::from_session(&session).await?;
    Ok(HtmlTemplate::new(HelloTemplate { page }))
}
