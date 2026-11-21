use super::repo;
use super::shared::{autocomplete_torneo, guild, resolver_torneo, uid};
use crate::{Context, Error, serenity};

/// Inscribe a un usuario como participante de un torneo.
#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
pub async fn inscribir(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_torneo"] torneo: String,
    usuario: serenity::User,
) -> Result<(), Error> {
    let Some(torneo) = resolver_torneo(ctx, &torneo).await? else {
        return Ok(());
    };
    let name = usuario.global_name.as_deref().unwrap_or(&usuario.name);
    let added = repo::join(&ctx.data().pool, guild(ctx)?, torneo, uid(&usuario)?, name).await?;
    ctx.say(if added {
        "Participante inscrito."
    } else {
        "El usuario ya está inscrito."
    })
    .await?;
    Ok(())
}
