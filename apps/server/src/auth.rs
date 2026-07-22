use askama::Template;
use axum::{
    Router,
    extract::Form,
    response::{IntoResponse, Sse, sse::Event},
    routing::{get, post},
};
use datastar::prelude::ExecuteScript;
use serde::Deserialize;
use std::convert::Infallible;
use tokio_stream::once;

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
    if password_text.trim().is_empty() {
        tracing::info!("Password is empty");
        return false;
    } else if password_text.chars().count() <= 8 {
        tracing::info!("Password is less than 8 characters long");
        return false;
    }
    true
}

// Handlers
async fn login_page() -> Result<impl IntoResponse, AppError> {
    Ok(HtmlTemplate::new(LoginTemplate {}))
}

async fn handle_login(Form(input): Form<LoginInput>) -> Result<impl IntoResponse, AppError> {
    tracing::info!("Inputs email={}, password={}", input.email, input.password);
    if verify_password(&input.password) {
        tracing::info!("Password verified");
        let event =
            ExecuteScript::new(r#"window.location.replace("/");"#).write_as_axum_sse_event();

        return Ok(Sse::new(once(Ok::<Event, Infallible>(event))));
    }

    let event =
        ExecuteScript::new(r#"console.log("Invalid credentials");"#).write_as_axum_sse_event();
    Ok(Sse::new(once(Ok::<Event, Infallible>(event))))
}
