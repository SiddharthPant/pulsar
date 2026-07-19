use std::time::Duration;

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header::CONTENT_TYPE},
    response::Response,
};
use http_body_util::BodyExt;
use server::{AppState, build_app};
use sqlx::PgPool;
use tower::util::ServiceExt;
use tower_http::normalize_path::NormalizePath;
use uuid::Uuid;

fn test_app(pool: PgPool) -> NormalizePath<Router> {
    build_app(AppState::new(pool), Duration::from_secs(1))
}

fn get(path: &str) -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri(path)
        .body(Body::empty())
        .expect("test request should be valid")
}

async fn body_text(response: Response) -> String {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("response body should be readable")
        .to_bytes();

    String::from_utf8(bytes.to_vec()).expect("response should contain UTF-8")
}

async fn seed_user(pool: &PgPool, full_name: &str, email: &str) -> Uuid {
    sqlx::query_scalar(
        r#"
        INSERT INTO users (full_name, email)
        VALUES ($1, $2)
        RETURNING id
        "#,
    )
    .bind(full_name)
    .bind(email)
    .fetch_one(pool)
    .await
    .expect("users should be inserted")
}

#[sqlx::test(migrations = "../../migrations")]
async fn empty_user_directory_shows_empty_state(pool: PgPool) {
    let response = test_app(pool)
        .oneshot(get("/users"))
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response
            .headers()
            .get(CONTENT_TYPE)
            .expect("content type should be present")
            .to_str()
            .expect("content type should be text"),
        "text/html; charset=utf-8"
    );

    let body = body_text(response).await;

    assert!(body.contains("User Directory"));
    assert!(body.contains("No registered users found in the system."));
}

#[sqlx::test(migrations = "../../migrations")]
async fn user_directory_renders_database_users(pool: PgPool) {
    let ada_id = seed_user(&pool, "Ada Lovelace", "ada@example.com").await;
    let grace_id = seed_user(&pool, "Grace Hopper", "grace@example.com").await;

    let response = test_app(pool)
        .oneshot(get("/users"))
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_text(response).await;

    assert!(body.contains("Ada Lovelace"));
    assert!(body.contains("Grace Hopper"));

    assert!(body.contains(&format!("/users/{ada_id}")));
    assert!(body.contains(&format!("/users/{grace_id}")));

    assert!(!body.contains("No registered users found"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn existing_user_detail_is_rendered(pool: PgPool) {
    let id = seed_user(&pool, "Ada Lovelace", "ada@example.com").await;

    let response = test_app(pool)
        .oneshot(get(&format!("/users/{id}")))
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_text(response).await;

    assert!(body.contains("User Account Details"));
    assert!(body.contains(&id.to_string()));
    assert!(body.contains("Ada Lovelace"));
    assert!(body.contains("ada@example.com"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn missing_user_returns_not_found(pool: PgPool) {
    let missing_id = Uuid::now_v7();

    let response = test_app(pool)
        .oneshot(get(&format!("/users/{missing_id}")))
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body = body_text(response).await;

    assert_eq!(body, format!("user {missing_id} does not exist"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn malformed_user_id_returns_bad_request(pool: PgPool) {
    let response = test_app(pool)
        .oneshot(get("/users/not-a-uuid"))
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../../migrations")]
async fn user_content_is_html_escaped(pool: PgPool) {
    let dangerous_name = "<script>alert('hello')</script>";
    let id = seed_user(&pool, dangerous_name, "attacker@example.com").await;

    let response = test_app(pool)
        .oneshot(get(&format!("/users/{id}")))
        .await
        .expect("request should complete");

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_text(response).await;

    assert!(!body.contains("<script>"));
    assert!(body.contains("alert"));
    assert!(body.contains("&#60;script&#62;"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn database_failure_returns_sanitized_internal_error(pool: PgPool) {
    let app = test_app(pool.clone());

    // PgPool clones share the same underlying pool. Closing one closes it
    // for the AppState stored inside the router as well.
    pool.close().await;

    let response = app
        .oneshot(get("/users"))
        .await
        .expect("Axum should still produce an HTTP response");

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body = body_text(response).await;

    assert_eq!(body, "Internal server error");
    assert!(!body.contains("pool"));
    assert!(!body.contains("database"));
    assert!(!body.contains("SQL"));
}

#[sqlx::test(migrations = "../../migrations")]
async fn user_directory_is_ordered_by_name(pool: PgPool) {
    // Insert in the opposite order to the expected output.
    seed_user(&pool, "Grace Hopper", "grace@example.com").await;
    seed_user(&pool, "Ada Lovelace", "ada@example.com").await;

    let response = test_app(pool)
        .oneshot(get("/users"))
        .await
        .expect("request should complete");

    let body = body_text(response).await;

    let ada_position = body.find("Ada Lovelace").expect("Ada should be rendered");
    let grace_position = body.find("Grace Hopper").expect("Grace should be rendered");

    assert!(ada_position < grace_position);
}
