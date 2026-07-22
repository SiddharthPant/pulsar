use crate::{auth::SessionUser, error::AppError};
use axum::{
    extract::FromRequestParts,
    http::request::Parts,
    response::{IntoResponse, Redirect, Response},
};
use tower_sessions::Session;

#[derive(Clone, Debug)]
pub struct PageContext {
    pub user_pid: Option<String>,
}

impl PageContext {
    pub async fn from_session(session: &Session) -> Result<Self, AppError> {
        let user_pid = session
            .get::<String>("user_pid")
            .await
            .map_err(anyhow::Error::from)?;

        Ok(Self { user_pid })
    }

    pub fn is_authenticated(&self) -> bool {
        self.user_pid.is_some()
    }
}

#[derive(Clone, Debug)]
pub struct LandingLayoutContext {
    authenticated: bool,
}

impl LandingLayoutContext {
    pub async fn from_session(session: &Session) -> Result<Self, AppError> {
        Ok(Self {
            authenticated: SessionUser::from_session(session).await?.is_some(),
        })
    }

    pub fn is_authenticated(&self) -> bool {
        self.authenticated
    }
}

#[derive(Clone, Debug)]
pub struct DashboardLayoutContext {
    pub user_pid: String,
    pub current_user_name: String,
}

impl<S> FromRequestParts<S> for DashboardLayoutContext
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session = Session::from_request_parts(parts, state)
            .await
            .map_err(IntoResponse::into_response)?;
        let user = SessionUser::from_session(&session)
            .await
            .map_err(IntoResponse::into_response)?
            .ok_or_else(|| Redirect::to("/auth/login").into_response())?;

        Ok(Self {
            user_pid: user.user_pid,
            current_user_name: user.display_name,
        })
    }
}
