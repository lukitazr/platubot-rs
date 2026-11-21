use super::shared::{autocomplete_torneo, guild, resolver_torneo};
use crate::{Context, Error, serenity};
use serenity::Mentionable;

/// Configura los canales para enviar y revisar capturas de un torneo.
#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
pub async fn capturas(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_torneo"] torneo: String,
    envio: serenity::GuildChannel,
    revision: serenity::GuildChannel,
) -> Result<(), Error> {
    let Some(id) = resolver_torneo(ctx, &torneo).await? else {
        return Ok(());
    };
    if envio.guild_id != revision.guild_id
        || Some(envio.guild_id) != ctx.guild_id()
        || envio.id == revision.id
        || envio.kind != serenity::ChannelType::Text
        || revision.kind != serenity::ChannelType::Text
    {
        ctx.say("Elige dos canales de texto diferentes de este servidor.")
            .await?;
        return Ok(());
    }
    let result = sqlx::query("UPDATE tournaments SET submit_channel_id=$1, review_channel_id=$2 WHERE tournament_id=$3 AND guild_id=$4 AND NOT finished")
        .bind(i64::try_from(envio.id.get())?).bind(i64::try_from(revision.id.get())?).bind(id).bind(guild(ctx)?).execute(&ctx.data().pool).await;
    match result {
        Ok(r) if r.rows_affected() == 1 => {
            ctx.say(format!(
                "Capturas configuradas: envío en {} y revisión en {}.",
                envio.id.mention(),
                revision.id.mention()
            ))
            .await?;
        }
        Ok(_) => {
            ctx.say("El torneo ya finalizó.").await?;
        }
        Err(sqlx::Error::Database(e)) if e.is_unique_violation() => {
            ctx.say("El canal de envío ya está asignado a otro torneo.")
                .await?;
        }
        Err(e) => return Err(e.into()),
    }
    Ok(())
}
