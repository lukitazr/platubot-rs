use super::repo;
use super::shared::{autocomplete_torneo, guild, resolver_torneo};
use crate::{Context, Error};

/// Muestra los participantes, la clasificación y los partidos de un torneo.
#[poise::command(slash_command)]
pub async fn ver(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_torneo"] torneo: String,
) -> Result<(), Error> {
    let Some(torneo) = resolver_torneo(ctx, &torneo).await? else {
        return Ok(());
    };
    let pool = &ctx.data().pool;
    let Some(t) = repo::get(pool, guild(ctx)?, torneo).await? else {
        ctx.say("Torneo no encontrado.").await?;
        return Ok(());
    };
    let participants = repo::participants(pool, torneo).await?;
    let matches = repo::matches(pool, torneo).await?;
    let mut out = format!(
        "**{}** (ID {}, {}) — {}\n",
        t.name,
        t.tournament_id,
        if t.oficial { "oficial" } else { "amistoso" },
        if t.finished { "finalizado" } else { "en curso" }
    );
    if let Some(champion) = t.champion_id {
        out.push_str(&format!("Campeón: <@{champion}>\n"));
    }
    out.push_str("\n**Clasificación** (3 puntos por victoria, 1 por empate)\n");
    for p in participants {
        out.push_str(&format!(
            "<@{}> — {} pts | {}V {}E {}D | {} WO | {}:{}\n",
            p.user_id,
            p.pg * 3 + p.pe,
            p.pg,
            p.pe,
            p.pp,
            p.wo,
            p.gf,
            p.gc
        ));
    }
    out.push_str("\n**Enfrentamientos**\n");
    for m in matches {
        out.push_str(&format!(
            "Fecha {}: <@{}> {} - {} <@{}>\n",
            m.fecha,
            m.home_id,
            m.home_goals.map_or("?".into(), |n| n.to_string()),
            m.away_goals.map_or("?".into(), |n| n.to_string()),
            m.away_id
        ));
    }
    // Discord messages are limited to 2,000 characters; split on line boundaries.
    let mut chunk = String::new();
    for line in out.lines() {
        if chunk.len() + line.len() + 1 > 1900 && !chunk.is_empty() {
            ctx.say(&chunk).await?;
            chunk.clear();
        }
        if line.len() > 1900 {
            continue;
        }
        chunk.push_str(line);
        chunk.push('\n');
    }
    if !chunk.is_empty() {
        ctx.say(chunk).await?;
    }
    Ok(())
}
