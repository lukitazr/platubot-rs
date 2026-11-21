use super::repo;
use super::shared::{
    autocomplete_participante, autocomplete_torneo, guild, resolver_participante, resolver_torneo,
};
use crate::{Context, Error};

/// Finaliza el torneo y asigna el título a un participante.
#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
pub async fn finalizar(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_torneo"] torneo: String,
    #[autocomplete = "autocomplete_participante"] campeon: String,
) -> Result<(), Error> {
    let Some(torneo) = resolver_torneo(ctx, &torneo).await? else {
        return Ok(());
    };
    let Some(campeon) = resolver_participante(ctx, torneo, &campeon).await? else {
        return Ok(());
    };
    let done = repo::finish(&ctx.data().pool, guild(ctx)?, torneo, campeon).await?;
    ctx.say(if done {
        "Torneo finalizado y título añadido al campeón."
    } else {
        "El campeón debe estar inscrito y no puede haber partidos pendientes."
    })
    .await?;
    Ok(())
}
