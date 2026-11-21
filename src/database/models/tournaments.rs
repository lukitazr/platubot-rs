use sqlx::FromRow;

#[derive(Debug, FromRow)]
pub struct Tournament {
    pub tournament_id: i64,
    pub guild_id: i64,
    pub name: String,
    pub oficial: bool,
    pub finished: bool,
    pub champion_id: Option<i64>,
}

#[derive(Debug, FromRow)]
pub struct Participant {
    pub user_id: i64,
    pub username: String,
    pub pg: i64,
    pub pe: i64,
    pub pp: i64,
    pub wo: i64,
    pub gf: i64,
    pub gc: i64,
}

#[derive(Debug, FromRow)]
pub struct Match {
    pub fecha: i32,
    pub home_id: i64,
    pub away_id: i64,
    pub home_goals: Option<i64>,
    pub away_goals: Option<i64>,
}
