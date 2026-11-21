use super::repo;
use super::shared::{
    autocomplete_participante, autocomplete_torneo, guild, resolver_participante, resolver_torneo,
};
use crate::{Context, Error};

/// Programa un enfrentamiento entre dos participantes en una fecha.
#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
pub async fn programar(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_torneo"] torneo: String,
    fecha: i32,
    #[autocomplete = "autocomplete_participante"] local: String,
    #[autocomplete = "autocomplete_participante"] visitante: String,
) -> Result<(), Error> {
    let Some(torneo) = resolver_torneo(ctx, &torneo).await? else {
        return Ok(());
    };
    let Some(local) = resolver_participante(ctx, torneo, &local).await? else {
        return Ok(());
    };
    let Some(visitante) = resolver_participante(ctx, torneo, &visitante).await? else {
        return Ok(());
    };
    if fecha <= 0 || local == visitante {
        ctx.say("La fecha debe ser positiva y los jugadores deben ser distintos.")
            .await?;
        return Ok(());
    }
    let added = repo::schedule(
        &ctx.data().pool,
        guild(ctx)?,
        torneo,
        fecha,
        local,
        visitante,
    )
    .await?;
    ctx.say(if added {
        "Enfrentamiento programado."
    } else {
        "Estos participantes ya tienen un enfrentamiento en esa fecha."
    })
    .await?;
    Ok(())
}
