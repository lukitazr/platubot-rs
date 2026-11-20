use sqlx::PgPool;
use crate::database::models::users::User;

pub async fn create_users(
    pool: &PgPool,
    id: i64,
    username: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("
        INSERT INTO users (id, username) 
        VALUES ($1, $2) 
        ON CONFLICT (id) DO NOTHING;
    ")
        .bind(id) 
        .bind(username)
        .execute(pool)
        .await?;

    Ok(())
}

pub async fn get_user(
    pool: &PgPool,
    id: i64,
) -> Result<Option<User>, sqlx::Error> {
    sqlx::query_as::<_, User>("
        SELECT id, username
        FROM users
        WHERE id = $1
    ")
        .bind(id) 
        .fetch_optional(pool)
        .await
}