use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = "../../migrations")]
async fn users_receive_database_generated_values(pool: PgPool) {
    let (id, has_created_at, has_updated_at): (Uuid, bool, bool) = sqlx::query_as(
        r#"
            INSERT INTO users (full_name, email)
            VALUES ($1, $2)
            RETURNING
                id,
                created_at IS NOT NULL,
                updated_at IS NOT NULL
            "#,
    )
    .bind("Ada Lovelace")
    .bind("ada@example.com")
    .fetch_one(&pool)
    .await
    .expect("user should be inserted");

    assert_ne!(id, Uuid::nil());
    assert!(has_created_at);
    assert!(has_updated_at);
}

#[sqlx::test(migrations = "../../migrations")]
async fn user_emails_must_be_unique(pool: PgPool) {
    sqlx::query(
        r#"
        INSERT INTO users (full_name, email)
        VALUES ($1, $2)
        "#,
    )
    .bind("First User")
    .bind("same@example.com")
    .execute(&pool)
    .await
    .expect("first insert should succeed");

    let error = sqlx::query(
        r#"
        INSERT INTO users (full_name, email)
        VALUES ($1, $2)
        "#,
    )
    .bind("Second User")
    .bind("same@example.com")
    .execute(&pool)
    .await
    .expect_err("duplicate email should be rejected");

    let error_code = error.as_database_error().and_then(|error| error.code());

    // PostgreSQL SQLSTATE 23505 means unique_violation.
    assert_eq!(error_code.as_deref(), Some("23505"));
}
