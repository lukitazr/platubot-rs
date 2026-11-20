use sqlx::PgPool;
use crate::database::models::guild_users::GuildUser;
use crate::database::repositories::users::get_user;

pub async fn create_guild_user(
    pool: &PgPool,
    guild_id: i64,
    user_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO guild_users (guild_id, user_id)
         VALUES ($1, $2)
         ON CONFLICT (guild_id, user_id) DO NOTHING"
    )
    .bind(guild_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    Ok(())
}

// pub async fn add_experience(
//     pool: &PgPool,
//     guild_id: i64,
//     user_id: i64,
//     amount: i64,
// ) -> Result<(), sqlx::Error> {

// }