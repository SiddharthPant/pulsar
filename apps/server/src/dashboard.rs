use crate::{AppState, page::DashboardLayoutContext, response::HtmlTemplate, users};
use askama::Template;
use axum::{Router, response::IntoResponse, routing::get};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .nest("/users", users::routes())
}

#[derive(Template)]
#[template(path = "pages/dashboard/index.html")]
struct DashboardTemplate {
    layout: DashboardLayoutContext,
}

async fn index(layout: DashboardLayoutContext) -> impl IntoResponse {
    HtmlTemplate::new(DashboardTemplate { layout })
}
