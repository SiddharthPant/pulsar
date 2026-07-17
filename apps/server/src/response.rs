use askama::Template;
use axum::response::{Html, IntoResponse, Response};

use crate::error::AppError;

pub struct HtmlTemplate<T>(T);

impl<T> HtmlTemplate<T> {
    pub fn new(template: T) -> Self {
        Self(template)
    }
}

impl<T> IntoResponse for HtmlTemplate<T>
where
    T: Template,
{
    fn into_response(self) -> Response {
        match self.0.render() {
            Ok(html) => Html(html).into_response(),
            Err(err) => AppError::Internal(err.into()).into_response(),
        }
    }
}
