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
use tower_sessions::Session;

use crate::{AppState, error::AppError, response::HtmlTemplate};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/login", get(login_page))
        .route("/login/handle", post(handle_login))
        .route("/logout", post(logout))
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

async fn handle_login(
    session: Session,
    Form(input): Form<LoginInput>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!("Inputs email={}, password={}", input.email, input.password);
    if verify_password(&input.password) {
        tracing::info!("Password verified");
        session.cycle_id().await.map_err(anyhow::Error::from)?;
        let user_pid = "usr_J9nrELBrwxfjhGmb";
        session
            .insert("user_pid", user_pid)
            .await
            .map_err(anyhow::Error::from)?;
        let event =
            ExecuteScript::new(r#"window.location.replace("/");"#).write_as_axum_sse_event();

        return Ok(Sse::new(once(Ok::<Event, Infallible>(event))));
    }

    let event =
        ExecuteScript::new(r#"console.log("Invalid credentials");"#).write_as_axum_sse_event();
    Ok(Sse::new(once(Ok::<Event, Infallible>(event))))
}

async fn logout(session: Session) -> Result<impl IntoResponse, AppError> {
    session.flush().await.map_err(anyhow::Error::from)?;
    let event = ExecuteScript::new(r#"window.location.replace("/");"#).write_as_axum_sse_event();
    Ok(Sse::new(once(Ok::<Event, Infallible>(event))))
}
