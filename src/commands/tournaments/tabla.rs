mod imagen;

use super::repo;
use super::shared::{autocomplete_torneo, descargar_avatares, guild, resolver_torneo};
use crate::{Context, Error, serenity};
use bot_command_macros::bot_command;

/// Muestra la clasificación y las estadísticas de los participantes.
#[bot_command(slash_command, guild_only)]
pub async fn tabla(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_torneo"]
    #[description = "Nombre o ID del torneo"]
    torneo: String,
) -> Result<(), Error> {
    ctx.defer().await?;
    let Some(torneo) = resolver_torneo(ctx, &torneo).await? else {
        return Ok(());
    };
    let pool = &ctx.data().pool;
    let Some(t) = repo::get(pool, guild(ctx)?, torneo).await? else {
        ctx.say("Torneo no encontrado.").await?;
        return Ok(());
    };
    let participants = repo::participants(pool, torneo).await?;
    if participants.is_empty() {
        ctx.say("El torneo no tiene participantes inscritos.")
            .await?;
        return Ok(());
    }

    let mut rows = Vec::with_capacity(participants.len());
    for participant in participants {
        let matches = repo::newest_user_matches(pool, torneo, participant.user_id).await?;
        rows.push((participant, matches));
    }
    let ids: Vec<_> = rows
        .iter()
        .map(|(participant, _)| participant.user_id)
        .collect();
    let avatars = descargar_avatares(ctx, &ids).await;
    let images = tokio::task::spawn_blocking(move || imagen::render(&t, &rows, &avatars)).await??;
    let pages = images.len();
    for (i, png) in images.into_iter().enumerate() {
        let page = i + 1;
        let attachment =
            serenity::CreateAttachment::bytes(png, format!("tabla-{torneo}-{page}.png"))
                .description(format!(
                    "Tabla de posiciones del torneo {torneo}. Página {page} de {pages}."
                ));
        ctx.send(poise::CreateReply::default().attachment(attachment))
            .await?;
    }
    Ok(())
}
