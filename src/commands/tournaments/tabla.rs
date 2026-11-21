use super::repo;
use super::shared::{autocomplete_torneo, guild, resolver_torneo};
use crate::database::models::tournaments::{Participant, Tournament};
use crate::{Context, Error};
use bot_command_macros::bot_command;

/// Muestra la clasificación y las estadísticas de los participantes.
#[bot_command(slash_command, guild_only)]
pub async fn tabla(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_torneo"]
    #[description = "Nombre o ID del torneo"]
    torneo: String,
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
    if participants.is_empty() {
        ctx.say("El torneo no tiene participantes inscritos.")
            .await?;
        return Ok(());
    }

    let out = format_standings(&t, &participants);

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

fn format_standings(t: &Tournament, participants: &[Participant]) -> String {
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
    for (i, p) in participants.iter().enumerate() {
        let dg = p.gf - p.gc;
        let dg_str = if dg > 0 {
            format!("+{dg}")
        } else {
            dg.to_string()
        };
        out.push_str(&format!(
            "{}. <@{}> — {} pts | {}V {}E {}D | {}:{} (DG {dg_str})\n",
            i + 1,
            p.user_id,
            p.pg * 3 + p.pe,
            p.pg,
            p.pe,
            p.pp,
            p.gf,
            p.gc
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_standings() {
        let t = Tournament {
            tournament_id: 10,
            guild_id: 1,
            name: "Supercopa".into(),
            oficial: false,
            finished: true,
            champion_id: Some(201),
        };
        let participants = vec![
            Participant {
                user_id: 201,
                username: "champion".into(),
                pg: 2,
                pe: 0,
                pp: 0,
                gf: 5,
                gc: 1,
            },
            Participant {
                user_id: 202,
                username: "runnerup".into(),
                pg: 0,
                pe: 0,
                pp: 2,
                gf: 1,
                gc: 5,
            },
        ];

        let standings = format_standings(&t, &participants);
        assert!(standings.contains("**Supercopa** (ID 10, amistoso) — finalizado"));
        assert!(standings.contains("Campeón: <@201>"));
        assert!(standings.contains("1. <@201> — 6 pts | 2V 0E 0D | 5:1 (DG +4)"));
        assert!(standings.contains("2. <@202> — 0 pts | 0V 0E 2D | 1:5 (DG -4)"));
    }
}
