use anyhow::Context;
use askama::Template;
use axum::{
    Router,
    extract::{Path, State},
    response::IntoResponse,
    routing::get,
};
use uuid::Uuid;

use crate::{AppState, error::AppError, response::HtmlTemplate};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_users))
        .route("/{id}", get(get_user))
}

#[derive(sqlx::FromRow)]
struct User {
    id: Uuid,
    full_name: String,
    email: String,
}

#[derive(Template)]
#[template(path = "user_list.html")]
struct UserListTemplate {
    users: Vec<User>,
}

#[derive(Template)]
#[template(path = "user_detail.html")]
struct UserDetailTemplate {
    user: User,
}

async fn list_users(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let users = sqlx::query_as!(
        User,
        r#"select id, full_name, email from users order by full_name, id"#
    )
    .fetch_all(&state.pool)
    .await
    .context("Failed to retrieve users from database")?;
    Ok(HtmlTemplate::new(UserListTemplate { users }))
}

async fn get_user(
    Path(id): Path<Uuid>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let user = sqlx::query_as!(
        User,
        "select id, full_name, email FROM users WHERE id = $1",
        id
    )
    .fetch_optional(&state.pool)
    .await
    .with_context(|| format!("failed to retrieve user {id}"))?
    .ok_or_else(|| AppError::NotFound(format!("user {id} does not exist")))?;

    Ok(HtmlTemplate::new(UserDetailTemplate { user }))
}
