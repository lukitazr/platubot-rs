use sqlx::{PgPool, Postgres, Transaction};
use crate::database::models::tournaments::{Match, Participant, Tournament};

pub async fn create(pool: &PgPool, guild: i64, name: &str, oficial: bool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("INSERT INTO tournaments (guild_id, name, oficial) VALUES ($1, $2, $3) RETURNING tournament_id")
        .bind(guild).bind(name).bind(oficial).fetch_one(pool).await
}

pub async fn get(pool: &PgPool, guild: i64, id: i64) -> Result<Option<Tournament>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM tournaments WHERE tournament_id = $1 AND guild_id = $2")
        .bind(id).bind(guild).fetch_optional(pool).await
}

pub async fn search(pool: &PgPool, guild: i64, prefix: &str, limit: i64) -> Result<Vec<(i64, String)>, sqlx::Error> {
    sqlx::query_as("SELECT tournament_id, name FROM tournaments WHERE guild_id = $1 AND (left(lower(name), char_length($2)) = lower($2) OR left(tournament_id::text, char_length($2)) = $2) ORDER BY tournament_id DESC LIMIT $3")
        .bind(guild).bind(prefix).bind(limit).fetch_all(pool).await
}

pub async fn search_name(pool: &PgPool, guild: i64, prefix: &str) -> Result<Vec<(i64, String)>, sqlx::Error> {
    sqlx::query_as("SELECT tournament_id, name FROM tournaments WHERE guild_id = $1 AND left(lower(name), char_length($2)) = lower($2) ORDER BY tournament_id DESC LIMIT 2")
        .bind(guild).bind(prefix).fetch_all(pool).await
}

async fn lock_open(tx: &mut Transaction<'_, Postgres>, guild: i64, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT tournament_id FROM tournaments WHERE tournament_id = $1 AND guild_id = $2 AND NOT finished FOR UPDATE")
        .bind(id).bind(guild).fetch_one(&mut **tx).await?;
    Ok(())
}

pub async fn join(pool: &PgPool, guild: i64, id: i64, user: i64, name: &str) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    lock_open(&mut tx, guild, id).await?;
    sqlx::query("INSERT INTO users (id, username) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET username = EXCLUDED.username")
        .bind(user).bind(name).execute(&mut *tx).await?;
    let result = sqlx::query("INSERT INTO tournament_participants (tournament_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(id).bind(user).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(result.rows_affected() == 1)
}

pub async fn schedule(pool: &PgPool, guild: i64, id: i64, fecha: i32, home: i64, away: i64) -> Result<i64, sqlx::Error> {
    let mut tx = pool.begin().await?;
    lock_open(&mut tx, guild, id).await?;
    let match_id = sqlx::query_scalar("INSERT INTO tournament_matches (tournament_id, fecha, home_id, away_id) VALUES ($1, $2, $3, $4) RETURNING match_id")
        .bind(id).bind(fecha).bind(home).bind(away).fetch_one(&mut *tx).await?;
    tx.commit().await?;
    Ok(match_id)
}

pub async fn result(pool: &PgPool, guild: i64, id: i64, match_id: i64, home_goals: i64, away_goals: i64, walkover: Option<i64>) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    lock_open(&mut tx, guild, id).await?;
    let game: Option<(i64, i64)> = sqlx::query_as("UPDATE tournament_matches SET home_goals = $1, away_goals = $2 WHERE match_id = $3 AND tournament_id = $4 AND home_goals IS NULL RETURNING home_id, away_id")
        .bind(home_goals).bind(away_goals).bind(match_id).bind(id).fetch_optional(&mut *tx).await?;
    let Some((home, away)) = game else { return Ok(false); };
    if let Some(loser) = walkover {
        if !((loser == home && home_goals < away_goals) || (loser == away && away_goals < home_goals)) { return Ok(false); }
        sqlx::query("UPDATE users SET wo=wo+1 WHERE id=$1").bind(loser).execute(&mut *tx).await?;
    }
    for (user, gf, gc) in [(home, home_goals, away_goals), (away, away_goals, home_goals)] {
        let pg = i64::from(gf > gc); let pe = i64::from(gf == gc); let pp = i64::from(gf < gc);
        sqlx::query("UPDATE tournament_participants SET pg=pg+$1, pe=pe+$2, pp=pp+$3, gf=gf+$4, gc=gc+$5 WHERE tournament_id=$6 AND user_id=$7")
            .bind(pg).bind(pe).bind(pp).bind(gf).bind(gc).bind(id).bind(user).execute(&mut *tx).await?;
        sqlx::query("UPDATE users SET pg=pg+$1, pe=pe+$2, pp=pp+$3, gf=gf+$4, gc=gc+$5 WHERE id=$6")
            .bind(pg).bind(pe).bind(pp).bind(gf).bind(gc).bind(user).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(true)
}

pub async fn participants(pool: &PgPool, id: i64) -> Result<Vec<Participant>, sqlx::Error> {
    sqlx::query_as("SELECT p.user_id, u.username, p.pg, p.pe, p.pp, p.gf, p.gc FROM tournament_participants p JOIN users u ON u.id=p.user_id WHERE p.tournament_id=$1 ORDER BY (p.pg * 3 + p.pe) DESC, (p.gf - p.gc) DESC, p.gf DESC, p.user_id")
        .bind(id).fetch_all(pool).await
}

pub async fn matches(pool: &PgPool, id: i64) -> Result<Vec<Match>, sqlx::Error> {
    sqlx::query_as("SELECT match_id, fecha, home_id, away_id, home_goals, away_goals FROM tournament_matches WHERE tournament_id=$1 ORDER BY fecha, match_id")
        .bind(id).fetch_all(pool).await
}

pub async fn finish(pool: &PgPool, guild: i64, id: i64, champion: i64) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    lock_open(&mut tx, guild, id).await?;
    let eligible: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tournament_participants WHERE tournament_id=$1 AND user_id=$2) AND NOT EXISTS(SELECT 1 FROM tournament_matches WHERE tournament_id=$1 AND home_goals IS NULL)")
        .bind(id).bind(champion).fetch_one(&mut *tx).await?;
    if !eligible { return Ok(false); }
    let oficial: bool = sqlx::query_scalar("UPDATE tournaments SET finished=TRUE, champion_id=$1 WHERE tournament_id=$2 RETURNING oficial")
        .bind(champion).bind(id).fetch_one(&mut *tx).await?;
    let query = if oficial { "UPDATE users SET titulos_oficiales=titulos_oficiales+1 WHERE id=$1" } else { "UPDATE users SET titulos_amistosos=titulos_amistosos+1 WHERE id=$1" };
    sqlx::query(query).bind(champion).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(true)
}
