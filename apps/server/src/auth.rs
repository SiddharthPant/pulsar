use askama::Template;
use axum::{
    Router,
    extract::Form,
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use serde::Deserialize;

use crate::{AppState, error::AppError, response::HtmlTemplate};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(login_page))
        .route("/handle", post(handle_login))
}

// Templates, Inputs and other structs
#[derive(Template)]
#[template(path = "auth/login.html")]
struct LoginTemplate;

#[derive(Debug, Deserialize)]
struct LoginInput {
    email: String,
    password: String,
}

// Functions
fn verify_password(password_text: &str) -> bool {
    if !password_text.is_empty() {
        return true;
    }
    tracing::info!("No password given");
    false
}

// Handlers
async fn login_page() -> Result<impl IntoResponse, AppError> {
    Ok(HtmlTemplate::new(LoginTemplate {}))
}

async fn handle_login(Form(input): Form<LoginInput>) -> Result<impl IntoResponse, AppError> {
    tracing::info!("Inputs email={}, password={}", input.email, input.password);
    if verify_password(&input.password) {
        tracing::info!("Password verified");
    }
    Ok(Redirect::to("/"))
}
