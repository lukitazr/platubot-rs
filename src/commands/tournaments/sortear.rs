use super::repo;
use super::shared::{autocomplete_torneo, guild, resolver_torneo};
use crate::{Context, Error};

/// Genera el calendario de todos contra todos para un torneo.
#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
pub async fn sortear(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_torneo"] torneo: String,
    #[description = "Cantidad de rondas (por defecto 1)"] rondas: Option<u32>,
) -> Result<(), Error> {
    let Some(torneo) = resolver_torneo(ctx, &torneo).await? else {
        return Ok(());
    };
    let rondas = rondas.unwrap_or(1);
    if !(1..=20).contains(&rondas) {
        ctx.say("La cantidad de rondas debe estar entre 1 y 20.")
            .await?;
        return Ok(());
    }
    ctx.defer().await?;
    let response = match repo::draw(&ctx.data().pool, guild(ctx)?, torneo, rondas as usize).await {
        Ok(repo::DrawResult::Created { dates, matches }) => format!("Sorteo creado: {dates} fechas y {matches} partidos en {rondas} ronda(s)."),
        Ok(repo::DrawResult::ExistingMatches) => "El torneo ya tiene partidos; no se ha modificado el calendario.".into(),
        Ok(repo::DrawResult::TooFewParticipants) => "Se necesitan al menos dos participantes inscritos.".into(),
        Ok(repo::DrawResult::UnequalHomeAway) => "Con un número par de participantes se necesita un número par de rondas para igualar los partidos de local y visitante.".into(),
        Ok(repo::DrawResult::UnsupportedSize) => "El sorteo admite hasta 64 participantes y entre 1 y 20 rondas.".into(),
        Ok(repo::DrawResult::NoSchedule) => "No se encontró un calendario válido; prueba nuevamente.".into(),
        Err(sqlx::Error::RowNotFound) => "El torneo ya no existe o está finalizado.".into(),
        Err(error) => return Err(error.into()),
    };
    ctx.say(response).await?;
    Ok(())
}
