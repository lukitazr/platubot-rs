use super::repo;
use super::shared::{
    autocomplete_participante, autocomplete_torneo, describir_resultado, guild,
    resolver_participante, resolver_torneo,
};
use crate::{Context, Error};

/// Registra el resultado de un enfrentamiento ya programado.
#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
pub async fn resultado(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_torneo"] torneo: String,
    fecha: i32,
    #[autocomplete = "autocomplete_participante"] jugador1: String,
    #[autocomplete = "autocomplete_participante"] jugador2: String,
    goles_jugador1: i64,
    goles_jugador2: i64,
) -> Result<(), Error> {
    let Some(torneo) = resolver_torneo(ctx, &torneo).await? else {
        return Ok(());
    };
    let Some(jugador1) = resolver_participante(ctx, torneo, &jugador1).await? else {
        return Ok(());
    };
    let Some(jugador2) = resolver_participante(ctx, torneo, &jugador2).await? else {
        return Ok(());
    };
    if fecha <= 0 || jugador1 == jugador2 || goles_jugador1 < 0 || goles_jugador2 < 0 {
        ctx.say("La fecha debe ser positiva, los jugadores distintos y los goles no negativos.")
            .await?;
        return Ok(());
    }
    let outcome = repo::result(
        &ctx.data().pool,
        guild(ctx)?,
        torneo,
        fecha,
        jugador1,
        jugador2,
        goles_jugador1,
        goles_jugador2,
        None,
    )
    .await?;
    ctx.say(describir_resultado(outcome, false)).await?;
    Ok(())
}
