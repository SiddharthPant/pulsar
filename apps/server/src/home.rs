use crate::{AppState, error::AppError, page::LandingLayoutContext, response::HtmlTemplate};
use askama::Template;
use axum::{Router, response::IntoResponse, routing::get};
use tower_sessions::Session;

pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(home))
}

#[derive(Template)]
#[template(path = "pages/index.html")]
struct HomeTemplate {
    layout: LandingLayoutContext,
}

async fn home(session: Session) -> Result<impl IntoResponse, AppError> {
    let layout = LandingLayoutContext::from_session(&session).await?;
    Ok(HtmlTemplate::new(HomeTemplate { layout }))
}
