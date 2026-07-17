use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub enum AppError {
    NotFound(String),
    Internal(anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound(message) => (StatusCode::NOT_FOUND, message).into_response(),
            Self::Internal(err) => {
                tracing::error!(error = ?err, "request failed");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
        }
    }
}

impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        Self::Internal(err)
    }
}
