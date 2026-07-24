use crate::error::AppError;
use askama::Template;
use axum::response::{Html, IntoResponse, Response, Sse, sse::Event};
use std::convert::Infallible;
use tokio_stream::once;

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

pub fn datastar_event(event: Event) -> Response {
    Sse::new(once(Ok::<Event, Infallible>(event))).into_response()
}
