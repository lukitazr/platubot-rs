use super::submissions::{admin, text_response, value};
use crate::{Error, serenity as s};
use sqlx::PgPool;

pub async fn interaction(
    ctx: &s::Context,
    interaction: &s::Interaction,
    pool: &PgPool,
) -> Result<(), Error> {
    let s::Interaction::Modal(modal) = interaction else {
        return Ok(());
    };
    let mut parts = modal.data.custom_id.split(':');
    if parts.next() != Some("result") {
        return Ok(());
    }
    let (Some(tournament), Some(game), None) = (
        parts.next().and_then(|x| x.parse::<i64>().ok()),
        parts.next().and_then(|x| x.parse::<i64>().ok()),
        parts.next(),
    ) else {
        return Ok(());
    };
    let Some(guild) = modal.guild_id else {
        return Ok(());
    };
    if !admin(&modal.member) {
        modal
            .create_response(ctx, text_response("Necesitas Administrar servidor."))
            .await?;
        return Ok(());
    }
    let scores = value(modal, "home")
        .and_then(|v| v.trim().parse::<i64>().ok())
        .zip(value(modal, "away").and_then(|v| v.trim().parse::<i64>().ok()))
        .filter(|(a, b)| *a >= 0 && *b >= 0);
    let Some((home_goals, away_goals)) = scores else {
        modal.create_response(ctx, text_response("Los goles deben ser números enteros no negativos. Ejecuta el comando nuevamente.")).await?;
        return Ok(());
    };
    let guild = i64::try_from(guild.get())?;
    let found: Option<(i32,i64,i64)> = sqlx::query_as("SELECT m.fecha,m.home_id,m.away_id FROM tournament_matches m JOIN tournaments t ON t.tournament_id=m.tournament_id WHERE t.guild_id=$1 AND t.tournament_id=$2 AND NOT t.finished AND m.match_id=$3 AND m.home_goals IS NULL").bind(guild).bind(tournament).bind(game).fetch_optional(pool).await?;
    let Some((fecha, home, away)) = found else {
        modal
            .create_response(
                ctx,
                text_response("El partido ya tiene resultado o el torneo no está activo."),
            )
            .await?;
        return Ok(());
    };
    let result = crate::database::repositories::tournaments::result_match(
        pool, guild, tournament, game, fecha, home, away, home_goals, away_goals,
    )
    .await;
    let response = match result {
        Ok(crate::database::repositories::tournaments::MatchResult::Saved) => format!(
            "Resultado registrado: <@{home}> {home_goals} – {away_goals} <@{away}>. Estadísticas actualizadas."
        ),
        Ok(_) | Err(sqlx::Error::RowNotFound) => {
            "El partido ya tiene resultado o el torneo no está activo.".into()
        }
        Err(error) => return Err(error.into()),
    };
    modal.create_response(ctx, text_response(response)).await?;
    Ok(())
}
