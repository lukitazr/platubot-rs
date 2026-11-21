use crate::database::models::tournaments::{Match, Participant, Tournament};
use sqlx::{PgPool, Postgres, Transaction};

pub enum DrawResult {
    Created { dates: usize, matches: usize },
    ExistingMatches,
    TooFewParticipants,
    UnequalHomeAway,
    UnsupportedSize,
    NoSchedule,
}

pub async fn draw(
    pool: &PgPool,
    guild: i64,
    id: i64,
    rounds: usize,
) -> Result<DrawResult, sqlx::Error> {
    let mut tx = pool.begin().await?;
    lock_open(&mut tx, guild, id).await?;
    let existing: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM tournament_matches WHERE tournament_id = $1)",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;
    if existing {
        return Ok(DrawResult::ExistingMatches);
    }
    let players: Vec<i64> = sqlx::query_scalar(
        "SELECT user_id FROM tournament_participants WHERE tournament_id = $1 ORDER BY user_id",
    )
    .bind(id)
    .fetch_all(&mut *tx)
    .await?;
    if players.len() < 2 {
        return Ok(DrawResult::TooFewParticipants);
    }
    if players.len() > 64 || rounds == 0 || rounds > 20 {
        return Ok(DrawResult::UnsupportedSize);
    }
    if players.len() % 2 == 0 && rounds % 2 == 1 {
        return Ok(DrawResult::UnequalHomeAway);
    }
    let schedule =
        tokio::task::spawn_blocking(move || crate::tournaments::draw::draw(&players, rounds))
            .await
            .map_err(|error| {
                sqlx::Error::Protocol(format!("Error al calcular el sorteo: {error}"))
            })?;
    let Some(schedule) = schedule else {
        return Ok(DrawResult::NoSchedule);
    };
    let dates = schedule.len();
    let mut matches = 0;
    for (date, games) in schedule.into_iter().enumerate() {
        for (home, away) in games {
            sqlx::query("INSERT INTO tournament_matches (tournament_id, fecha, home_id, away_id) VALUES ($1, $2, $3, $4)")
                .bind(id).bind((date + 1) as i32).bind(home).bind(away).execute(&mut *tx).await?;
            matches += 1;
        }
    }
    tx.commit().await?;
    Ok(DrawResult::Created { dates, matches })
}

pub async fn create(
    pool: &PgPool,
    guild: i64,
    name: &str,
    oficial: bool,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar("INSERT INTO tournaments (guild_id, name, oficial) VALUES ($1, $2, $3) RETURNING tournament_id")
        .bind(guild).bind(name).bind(oficial).fetch_one(pool).await
}

pub async fn get(pool: &PgPool, guild: i64, id: i64) -> Result<Option<Tournament>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM tournaments WHERE tournament_id = $1 AND guild_id = $2")
        .bind(id)
        .bind(guild)
        .fetch_optional(pool)
        .await
}

pub async fn search(
    pool: &PgPool,
    guild: i64,
    prefix: &str,
    limit: i64,
) -> Result<Vec<(i64, String)>, sqlx::Error> {
    sqlx::query_as("SELECT tournament_id, name FROM tournaments WHERE guild_id = $1 AND (left(lower(name), char_length($2)) = lower($2) OR left(tournament_id::text, char_length($2)) = $2) ORDER BY tournament_id DESC LIMIT $3")
        .bind(guild).bind(prefix).bind(limit).fetch_all(pool).await
}

pub async fn search_name(
    pool: &PgPool,
    guild: i64,
    prefix: &str,
) -> Result<Vec<(i64, String)>, sqlx::Error> {
    sqlx::query_as("SELECT tournament_id, name FROM tournaments WHERE guild_id = $1 AND left(lower(name), char_length($2)) = lower($2) ORDER BY tournament_id DESC LIMIT 2")
        .bind(guild).bind(prefix).fetch_all(pool).await
}

pub async fn search_participants(
    pool: &PgPool,
    guild: i64,
    tournament: i64,
    prefix: &str,
) -> Result<Vec<(i64, String)>, sqlx::Error> {
    sqlx::query_as("SELECT u.id, u.username FROM tournament_participants p JOIN tournaments t ON t.tournament_id = p.tournament_id JOIN users u ON u.id = p.user_id WHERE t.guild_id = $1 AND p.tournament_id = $2 AND (left(lower(u.username), char_length($3)) = lower($3) OR left(u.id::text, char_length($3)) = $3) ORDER BY lower(u.username), u.id LIMIT 25")
        .bind(guild).bind(tournament).bind(prefix).fetch_all(pool).await
}

pub async fn is_participant(
    pool: &PgPool,
    guild: i64,
    tournament: i64,
    user: i64,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tournament_participants p JOIN tournaments t ON t.tournament_id = p.tournament_id WHERE t.guild_id = $1 AND p.tournament_id = $2 AND p.user_id = $3)")
        .bind(guild).bind(tournament).bind(user).fetch_one(pool).await
}

async fn lock_open(
    tx: &mut Transaction<'_, Postgres>,
    guild: i64,
    id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT tournament_id FROM tournaments WHERE tournament_id = $1 AND guild_id = $2 AND NOT finished FOR UPDATE")
        .bind(id).bind(guild).fetch_one(&mut **tx).await?;
    Ok(())
}

pub async fn join(
    pool: &PgPool,
    guild: i64,
    id: i64,
    user: i64,
    name: &str,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    lock_open(&mut tx, guild, id).await?;
    sqlx::query("INSERT INTO users (id, username) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET username = EXCLUDED.username")
        .bind(user).bind(name).execute(&mut *tx).await?;
    let result = sqlx::query("INSERT INTO tournament_participants (tournament_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(id).bind(user).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(result.rows_affected() == 1)
}

pub async fn schedule(
    pool: &PgPool,
    guild: i64,
    id: i64,
    fecha: i32,
    home: i64,
    away: i64,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    lock_open(&mut tx, guild, id).await?;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tournament_matches WHERE tournament_id = $1 AND fecha = $2 AND ((home_id = $3 AND away_id = $4) OR (home_id = $4 AND away_id = $3)))")
        .bind(id).bind(fecha).bind(home).bind(away).fetch_one(&mut *tx).await?;
    if exists {
        return Ok(false);
    }
    sqlx::query("INSERT INTO tournament_matches (tournament_id, fecha, home_id, away_id) VALUES ($1, $2, $3, $4)")
        .bind(id).bind(fecha).bind(home).bind(away).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(true)
}

pub enum MatchResult {
    Saved,
    AlreadyPlayed,
    InvalidWalkover,
    Ambiguous,
    Closest {
        fecha: i32,
        home: i64,
        away: i64,
        played: bool,
    },
    NotFound,
}

pub async fn result(
    pool: &PgPool,
    guild: i64,
    id: i64,
    fecha: i32,
    player1: i64,
    player2: i64,
    goals1: i64,
    goals2: i64,
    walkover: Option<i64>,
) -> Result<MatchResult, sqlx::Error> {
    result_inner(
        pool, guild, id, fecha, player1, player2, goals1, goals2, walkover, None, None,
    )
    .await
}

pub async fn result_submission(
    pool: &PgPool,
    guild: i64,
    id: i64,
    fecha: i32,
    home: i64,
    away: i64,
    goals1: i64,
    goals2: i64,
    submission: i64,
) -> Result<MatchResult, sqlx::Error> {
    result_inner(
        pool,
        guild,
        id,
        fecha,
        home,
        away,
        goals1,
        goals2,
        None,
        Some(submission),
        None,
    )
    .await
}

pub async fn result_match(
    pool: &PgPool,
    guild: i64,
    tournament: i64,
    match_id: i64,
    fecha: i32,
    home: i64,
    away: i64,
    home_goals: i64,
    away_goals: i64,
) -> Result<MatchResult, sqlx::Error> {
    result_inner(
        pool,
        guild,
        tournament,
        fecha,
        home,
        away,
        home_goals,
        away_goals,
        None,
        None,
        Some(match_id),
    )
    .await
}

async fn result_inner(
    pool: &PgPool,
    guild: i64,
    id: i64,
    fecha: i32,
    player1: i64,
    player2: i64,
    goals1: i64,
    goals2: i64,
    walkover: Option<i64>,
    submission: Option<i64>,
    selected_match: Option<i64>,
) -> Result<MatchResult, sqlx::Error> {
    let mut tx = pool.begin().await?;
    lock_open(&mut tx, guild, id).await?;
    if let Some(submission) = submission {
        let updated = sqlx::query("UPDATE match_submissions SET status='approved' WHERE submission_id=$1 AND tournament_id=$2 AND status='pending'").bind(submission).bind(id).execute(&mut *tx).await?;
        if updated.rows_affected() == 0 {
            return Ok(MatchResult::AlreadyPlayed);
        }
    }
    let games: Vec<(i64, i64, i64, Option<i64>)> = sqlx::query_as("SELECT match_id, home_id, away_id, home_goals FROM tournament_matches WHERE tournament_id = $1 AND fecha = $2 AND ((home_id = $3 AND away_id = $4) OR (home_id = $4 AND away_id = $3)) AND ($5::bigint IS NULL OR match_id = (SELECT match_id FROM match_submissions WHERE submission_id=$5)) AND ($6::bigint IS NULL OR match_id=$6) ORDER BY match_id LIMIT 2")
        .bind(id).bind(fecha).bind(player1).bind(player2).bind(submission).bind(selected_match).fetch_all(&mut *tx).await?;
    if games.len() > 1 {
        return Ok(MatchResult::Ambiguous);
    }
    let Some(&(match_id, home, away, played)) = games.first() else {
        let closest: Option<(i32, i64, i64, bool)> = sqlx::query_as("SELECT fecha, home_id, away_id, home_goals IS NOT NULL FROM tournament_matches WHERE tournament_id = $1 AND ((home_id = $3 AND away_id = $4) OR (home_id = $4 AND away_id = $3)) ORDER BY abs(fecha::bigint - $2::bigint), fecha, match_id LIMIT 1")
            .bind(id).bind(fecha).bind(player1).bind(player2).fetch_optional(&mut *tx).await?;
        return Ok(match closest {
            Some((fecha, home, away, played)) => MatchResult::Closest {
                fecha,
                home,
                away,
                played,
            },
            None => MatchResult::NotFound,
        });
    };
    if played.is_some() {
        return Ok(MatchResult::AlreadyPlayed);
    }
    let (home_goals, away_goals) = if home == player1 {
        (goals1, goals2)
    } else {
        (goals2, goals1)
    };
    if let Some(loser) = walkover {
        if !((loser == home && home_goals < away_goals)
            || (loser == away && away_goals < home_goals))
        {
            return Ok(MatchResult::InvalidWalkover);
        }
        sqlx::query("UPDATE users SET wo=wo+1 WHERE id=$1")
            .bind(loser)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "UPDATE tournament_participants SET wo=wo+1 WHERE tournament_id=$1 AND user_id=$2",
        )
        .bind(id)
        .bind(loser)
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query(
        "UPDATE tournament_matches SET home_goals = $1, away_goals = $2, played_at = NOW() WHERE match_id = $3",
    )
    .bind(home_goals)
    .bind(away_goals)
    .bind(match_id)
    .execute(&mut *tx)
    .await?;
    for (user, gf, gc) in [
        (home, home_goals, away_goals),
        (away, away_goals, home_goals),
    ] {
        let pg = i64::from(gf > gc);
        let pe = i64::from(gf == gc);
        let pp = i64::from(gf < gc);
        sqlx::query("UPDATE tournament_participants SET pg=pg+$1, pe=pe+$2, pp=pp+$3, gf=gf+$4, gc=gc+$5 WHERE tournament_id=$6 AND user_id=$7")
            .bind(pg).bind(pe).bind(pp).bind(gf).bind(gc).bind(id).bind(user).execute(&mut *tx).await?;
        sqlx::query(
            "UPDATE users SET pg=pg+$1, pe=pe+$2, pp=pp+$3, gf=gf+$4, gc=gc+$5 WHERE id=$6",
        )
        .bind(pg)
        .bind(pe)
        .bind(pp)
        .bind(gf)
        .bind(gc)
        .bind(user)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(MatchResult::Saved)
}

pub async fn participants(pool: &PgPool, id: i64) -> Result<Vec<Participant>, sqlx::Error> {
    sqlx::query_as("SELECT p.user_id, u.username, p.pg, p.pe, p.pp, p.wo, p.gf, p.gc FROM tournament_participants p JOIN users u ON u.id=p.user_id WHERE p.tournament_id=$1 ORDER BY (p.pg * 3 + p.pe) DESC, (p.gf - p.gc) DESC, p.gf DESC, p.wo, p.user_id")
        .bind(id).fetch_all(pool).await
}

pub async fn matches(pool: &PgPool, id: i64) -> Result<Vec<Match>, sqlx::Error> {
    sqlx::query_as("SELECT fecha, home_id, away_id, home_goals, away_goals FROM tournament_matches WHERE tournament_id=$1 ORDER BY fecha, match_id")
        .bind(id).fetch_all(pool).await
}

pub async fn newest_user_matches(
    pool: &PgPool,
    id: i64,
    user: i64,
) -> Result<Vec<Match>, sqlx::Error> {
    sqlx::query_as("
    SELECT fecha, home_id, away_id, home_goals, away_goals FROM tournament_matches 
    WHERE tournament_id=$1 AND (home_id=$2 OR away_id=$2) AND played_at IS NOT NULL ORDER BY played_at DESC, fecha DESC, match_id DESC LIMIT 5")
        .bind(id).bind(user).fetch_all(pool).await
}

pub async fn finish(
    pool: &PgPool,
    guild: i64,
    id: i64,
    champion: i64,
) -> Result<bool, sqlx::Error> {
    let mut tx = pool.begin().await?;
    lock_open(&mut tx, guild, id).await?;
    let eligible: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM tournament_participants WHERE tournament_id=$1 AND user_id=$2) AND NOT EXISTS(SELECT 1 FROM tournament_matches WHERE tournament_id=$1 AND home_goals IS NULL)")
        .bind(id).bind(champion).fetch_one(&mut *tx).await?;
    if !eligible {
        return Ok(false);
    }
    let oficial: bool = sqlx::query_scalar("UPDATE tournaments SET finished=TRUE, champion_id=$1 WHERE tournament_id=$2 RETURNING oficial")
        .bind(champion).bind(id).fetch_one(&mut *tx).await?;
    let query = if oficial {
        "UPDATE users SET titulos_oficiales=titulos_oficiales+1 WHERE id=$1"
    } else {
        "UPDATE users SET titulos_amistosos=titulos_amistosos+1 WHERE id=$1"
    };
    sqlx::query(query).bind(champion).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test(migrations = "./migrations")]
    async fn selected_match_is_recorded_once_even_with_duplicate_fixtures(
        pool: PgPool,
    ) -> Result<(), sqlx::Error> {
        let tournament = create(&pool, 1, "Resultados", false).await?;
        for player in 1..=2_i64 {
            join(&pool, 1, tournament, player, &format!("jugador{player}")).await?;
        }
        let first: i64 = sqlx::query_scalar("INSERT INTO tournament_matches (tournament_id,fecha,home_id,away_id) VALUES ($1,1,1,2) RETURNING match_id").bind(tournament).fetch_one(&pool).await?;
        let second: i64 = sqlx::query_scalar("INSERT INTO tournament_matches (tournament_id,fecha,home_id,away_id) VALUES ($1,1,1,2) RETURNING match_id").bind(tournament).fetch_one(&pool).await?;
        assert!(matches!(
            result_match(&pool, 1, tournament, second, 1, 1, 2, 3, 1).await?,
            MatchResult::Saved
        ));
        assert!(matches!(
            result_match(&pool, 1, tournament, second, 1, 1, 2, 3, 1).await?,
            MatchResult::AlreadyPlayed
        ));
        let unplayed: Option<i64> =
            sqlx::query_scalar("SELECT home_goals FROM tournament_matches WHERE match_id=$1")
                .bind(first)
                .fetch_one(&pool)
                .await?;
        assert_eq!(unplayed, None);
        let (wins, goals): (i64, i64) = sqlx::query_as(
            "SELECT pg,gf FROM tournament_participants WHERE tournament_id=$1 AND user_id=1",
        )
        .bind(tournament)
        .fetch_one(&pool)
        .await?;
        assert_eq!((wins, goals), (1, 3));
        Ok(())
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn participants_use_walkovers_to_break_complete_ties(
        pool: PgPool,
    ) -> Result<(), sqlx::Error> {
        let tournament = create(&pool, 1, "Desempate por WO", false).await?;
        for (user_id, walkovers) in [(1_i64, 2_i64), (2, 0), (3, 1), (4, 0)] {
            sqlx::query("INSERT INTO users (id, username) VALUES ($1, $2)")
                .bind(user_id)
                .bind(format!("user{user_id}"))
                .execute(&pool)
                .await?;
            sqlx::query("INSERT INTO tournament_participants (tournament_id, user_id, pg, pe, pp, gf, gc, wo) VALUES ($1, $2, 1, 0, 0, 3, 1, $3)")
                .bind(tournament)
                .bind(user_id)
                .bind(walkovers)
                .execute(&pool)
                .await?;
        }

        let participants = participants(&pool, tournament).await?;
        let ordered_ids: Vec<_> = participants
            .iter()
            .map(|participant| participant.user_id)
            .collect();

        assert_eq!(ordered_ids, vec![2, 4, 3, 1]);
        Ok(())
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn results_are_identified_by_tournament_date_and_players(
        pool: PgPool,
    ) -> Result<(), sqlx::Error> {
        for id in 1..=3_i64 {
            sqlx::query("INSERT INTO users (id, username) VALUES ($1, $2)")
                .bind(id)
                .bind(format!("user{id}"))
                .execute(&pool)
                .await?;
        }
        let t1 = create(&pool, 1, "Primero", false).await?;
        let t2 = create(&pool, 1, "Segundo", false).await?;
        for t in [t1, t2] {
            for player in 1..=3 {
                sqlx::query(
                    "INSERT INTO tournament_participants (tournament_id, user_id) VALUES ($1,$2)",
                )
                .bind(t)
                .bind(player)
                .execute(&pool)
                .await?;
            }
        }
        assert!(schedule(&pool, 1, t1, 2, 1, 2).await?);
        assert!(!schedule(&pool, 1, t1, 2, 2, 1).await?);
        assert!(schedule(&pool, 1, t1, 6, 2, 1).await?);
        assert!(schedule(&pool, 1, t2, 2, 2, 1).await?);
        assert!(matches!(
            result(&pool, 1, t1, 3, 1, 2, 1, 0, None).await?,
            MatchResult::Closest {
                fecha: 2,
                home: 1,
                away: 2,
                played: false
            }
        ));
        assert!(matches!(
            result(&pool, 1, t1, 1, 1, 3, 1, 0, None).await?,
            MatchResult::NotFound
        ));
        assert!(matches!(
            result(&pool, 1, t1, 2, 2, 1, 4, 1, None).await?,
            MatchResult::Saved
        ));
        assert!(matches!(
            result(&pool, 1, t1, 2, 1, 2, 1, 4, None).await?,
            MatchResult::AlreadyPlayed
        ));
        let score: (Option<i64>, Option<i64>) = sqlx::query_as("SELECT home_goals, away_goals FROM tournament_matches WHERE tournament_id=$1 AND fecha=2").bind(t1).fetch_one(&pool).await?;
        assert_eq!(score, (Some(1), Some(4)));
        let played_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
            "SELECT played_at FROM tournament_matches WHERE tournament_id=$1 AND fecha=2",
        )
        .bind(t1)
        .fetch_one(&pool)
        .await?;
        assert!(played_at.is_some());
        let unplayed_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
            "SELECT played_at FROM tournament_matches WHERE tournament_id=$1 AND fecha=6",
        )
        .bind(t1)
        .fetch_one(&pool)
        .await?;
        assert!(unplayed_at.is_none());
        assert!(matches!(
            result(&pool, 1, t2, 2, 2, 1, 3, 0, Some(2)).await?,
            MatchResult::InvalidWalkover
        ));
        assert!(matches!(
            result(&pool, 1, t2, 2, 2, 1, 3, 0, Some(1)).await?,
            MatchResult::Saved
        ));
        let wo: i64 = sqlx::query_scalar("SELECT wo FROM users WHERE id=1")
            .fetch_one(&pool)
            .await?;
        assert_eq!(wo, 1);
        let tournament_wo: i64 = sqlx::query_scalar(
            "SELECT wo FROM tournament_participants WHERE tournament_id=$1 AND user_id=1",
        )
        .bind(t2)
        .fetch_one(&pool)
        .await?;
        assert_eq!(tournament_wo, 1);
        assert_eq!(wo, tournament_wo);
        let walkover_played_at: Option<chrono::DateTime<chrono::Utc>> = sqlx::query_scalar(
            "SELECT played_at FROM tournament_matches WHERE tournament_id=$1 AND fecha=2",
        )
        .bind(t2)
        .fetch_one(&pool)
        .await?;
        assert!(walkover_played_at.is_some());
        let t1_wo: i64 = sqlx::query_scalar(
            "SELECT wo FROM tournament_participants WHERE tournament_id=$1 AND user_id=1",
        )
        .bind(t1)
        .fetch_one(&pool)
        .await?;
        assert_eq!(t1_wo, 0);
        Ok(())
    }
}
