use super::repo;
use super::shared::guild;
use crate::{Context, Error};

/// Crea un torneo amistoso u oficial en este servidor.
#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
pub async fn crear(ctx: Context<'_>, nombre: String, oficial: bool) -> Result<(), Error> {
    let id = repo::create(&ctx.data().pool, guild(ctx)?, &nombre, oficial).await?;
    ctx.say(format!("Torneo **{nombre}** creado con ID `{id}`."))
        .await?;
    Ok(())
}
