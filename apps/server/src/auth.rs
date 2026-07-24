use askama::Template;
use axum::{
    Router,
    extract::Form,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use datastar::{patch_elements::PatchElements, prelude::ExecuteScript};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;

use crate::{
    AppState,
    error::AppError,
    response::{HtmlTemplate, datastar_event},
};

const AUTH_USER_KEY: &str = "auth.user";
const DASHBOARD_PATH: &str = "/dashboard";
const HOME_PATH: &str = "/";
const INVALID_CREDENTIALS: &str = "Invalid email or password";

// Templates, Inputs and other structs
#[derive(Template)]
#[template(path = "pages/auth/login.html")]
struct LoginTemplate {
    email: String,
    error: Option<&'static str>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SessionUser {
    pub user_pid: String,
    pub display_name: String,
}

#[derive(Debug, Deserialize)]
struct LoginInput {
    email: String,
    password: String,
}

// Functions
impl SessionUser {
    pub async fn from_session(session: &Session) -> Result<Option<Self>, AppError> {
        session
            .get(AUTH_USER_KEY)
            .await
            .map_err(anyhow::Error::from)
            .map_err(AppError::from)
    }
}

fn is_datastar_request(headers: &HeaderMap) -> bool {
    headers.contains_key("datastar-request")
}

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

// Routes
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/login", get(login_page).post(login))
        .route("/logout", post(logout))
}

// Handlers
async fn login_page(session: Session) -> Result<impl IntoResponse, AppError> {
    if SessionUser::from_session(&session).await?.is_some() {
        return Ok(Redirect::to(DASHBOARD_PATH).into_response());
    }
    Ok(HtmlTemplate::new(LoginTemplate {
        email: String::new(),
        error: None,
    })
    .into_response())
}

async fn login(
    session: Session,
    headers: HeaderMap,
    Form(input): Form<LoginInput>,
) -> Result<Response, AppError> {
    let email = input.email.trim();
    if !email.is_empty() && verify_password(&input.password) {
        let normalized_email = input.email.trim().to_lowercase();
        session.cycle_id().await.map_err(anyhow::Error::from)?;
        session
            .insert(
                AUTH_USER_KEY,
                SessionUser {
                    user_pid: "usr_J9nrELBrwxfjhGmb".to_owned(),
                    display_name: normalized_email,
                },
            )
            .await
            .map_err(anyhow::Error::from)?;
        if is_datastar_request(&headers) {
            let event = ExecuteScript::new(r#"window.location.replace("/dashboard");"#)
                .write_as_axum_sse_event();
            return Ok(datastar_event(event));
        }

        return Ok(Redirect::to(DASHBOARD_PATH).into_response());
    }

    if is_datastar_request(&headers) {
        let event = PatchElements::new(format!(
            r#"<p id="login-error" class="auth-form__error" role="alert">{INVALID_CREDENTIALS}</p>"#
        ))
        .write_as_axum_sse_event();

        return Ok(datastar_event(event));
    }

    Ok((
        StatusCode::UNPROCESSABLE_ENTITY,
        HtmlTemplate::new(LoginTemplate {
            email: input.email,
            error: Some(INVALID_CREDENTIALS),
        }),
    )
        .into_response())
}

async fn logout(session: Session, headers: HeaderMap) -> Result<impl IntoResponse, AppError> {
    session.flush().await.map_err(anyhow::Error::from)?;

    if is_datastar_request(&headers) {
        if is_datastar_request(&headers) {
            let event =
                ExecuteScript::new(r#"window.location.replace("/");"#).write_as_axum_sse_event();
            return Ok(datastar_event(event));
        }
    }

    Ok(Redirect::to(HOME_PATH).into_response())
}
