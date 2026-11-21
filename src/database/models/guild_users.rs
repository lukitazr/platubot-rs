use sqlx::{
    FromRow,
    types::chrono::{DateTime, Utc},
};

#[derive(Debug, FromRow)]
pub struct GuildUser {
    pub guild_id: i64,
    pub user_id: i64,
    pub level: i16,
    pub experience: i64,
    pub last_xp_at: Option<DateTime<Utc>>,
}
