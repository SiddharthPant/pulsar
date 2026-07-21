use askama::Template;
use axum::{Router, extract::State, routing::get, routing::post};

pub fn routes() -> Router<AppState> {
    Router::new().route("/login", get(login).post(login))
}

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate;

async fn login_page(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    Ok(HtmlTemplate::new(LoginTemplate {}))
}
