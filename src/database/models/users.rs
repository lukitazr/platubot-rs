use sqlx::FromRow;

// PostgreSQL BIGINT stores non-negative counters up to i64::MAX.
#[derive(Debug, FromRow)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub pg: i64,
    pub pe: i64,
    pub pp: i64,
    pub wo: i64,
    pub gf: i64,
    pub gc: i64,
    pub titulos_oficiales: i64,
    pub titulos_amistosos: i64,
}
