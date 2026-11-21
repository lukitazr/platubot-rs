use super::repo;
use super::shared::{autocomplete_torneo, guild, resolver_torneo};
use crate::database::models::tournaments::{Match, Tournament};
use crate::{Context, Error, serenity};
use bot_command_macros::bot_command;
use std::{collections::BTreeMap, time::Duration};

/// Muestra los enfrentamientos de un torneo, organizados por fecha.
#[bot_command(slash_command, guild_only)]
pub async fn fixture(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_torneo"]
    #[description = "Nombre o ID del torneo"]
    torneo: String,
    #[description = "Fecha inicial a consultar (opcional)"] fecha: Option<i32>,
) -> Result<(), Error> {
    let Some(torneo) = resolver_torneo(ctx, &torneo).await? else {
        return Ok(());
    };
    let pool = &ctx.data().pool;
    let Some(t) = repo::get(pool, guild(ctx)?, torneo).await? else {
        ctx.say("Torneo no encontrado.").await?;
        return Ok(());
    };
    let matches = repo::matches(pool, torneo).await?;
    if matches.is_empty() {
        ctx.say("El torneo no tiene enfrentamientos programados.")
            .await?;
        return Ok(());
    }

    let mut fechas_map = BTreeMap::<i32, Vec<Match>>::new();
    for m in matches {
        fechas_map.entry(m.fecha).or_default().push(m);
    }
    let fechas: Vec<(i32, Vec<Match>)> = fechas_map.into_iter().collect();
    let total_pages = fechas.len();

    let mut current_page = 0;
    if let Some(target_fecha) = fecha {
        if let Some(pos) = fechas.iter().position(|(f, _)| *f == target_fecha) {
            current_page = pos;
        } else {
            let available_msg = if total_pages == 1 {
                format!("la única fecha es la {}", fechas[0].0)
            } else {
                format!(
                    "fechas disponibles de la {} a la {}",
                    fechas.first().unwrap().0,
                    fechas.last().unwrap().0
                )
            };
            ctx.say(format!(
                "La fecha {target_fecha} no existe en este torneo ({available_msg})."
            ))
            .await?;
            return Ok(());
        }
    }

    if total_pages == 1 {
        let content = format_fixture_page(&t, fechas[0].0, 0, 1, &fechas[0].1);
        ctx.say(content).await?;
        return Ok(());
    }

    let ctx_id = ctx.id();
    let prev_button_id = format!("{ctx_id}_prev");
    let next_button_id = format!("{ctx_id}_next");

    let make_buttons = |page: usize, disabled: bool| {
        let indicator_id = format!("{ctx_id}_ind");
        serenity::CreateActionRow::Buttons(vec![
            serenity::CreateButton::new(&prev_button_id)
                .emoji('◀')
                .label("Anterior")
                .disabled(disabled),
            serenity::CreateButton::new(indicator_id)
                .label(format!("{}/{}", page + 1, total_pages))
                .style(serenity::ButtonStyle::Secondary)
                .disabled(true),
            serenity::CreateButton::new(&next_button_id)
                .emoji('▶')
                .label("Siguiente")
                .disabled(disabled),
        ])
    };

    let reply = poise::CreateReply::default()
        .content(format_fixture_page(
            &t,
            fechas[current_page].0,
            current_page,
            total_pages,
            &fechas[current_page].1,
        ))
        .components(vec![make_buttons(current_page, false)]);

    let reply_handle = ctx.send(reply).await?;

    while let Some(press) = serenity::collector::ComponentInteractionCollector::new(ctx)
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        .timeout(Duration::from_secs(600))
        .await
    {
        if press.data.custom_id == next_button_id {
            current_page = (current_page + 1) % total_pages;
        } else if press.data.custom_id == prev_button_id {
            current_page = current_page.checked_sub(1).unwrap_or(total_pages - 1);
        } else {
            continue;
        }

        let page_content = format_fixture_page(
            &t,
            fechas[current_page].0,
            current_page,
            total_pages,
            &fechas[current_page].1,
        );
        let buttons = make_buttons(current_page, false);

        press
            .create_response(
                ctx.serenity_context(),
                serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new()
                        .content(page_content)
                        .components(vec![buttons]),
                ),
            )
            .await?;
    }

    let disabled_buttons = make_buttons(current_page, true);
    let last_content = format_fixture_page(
        &t,
        fechas[current_page].0,
        current_page,
        total_pages,
        &fechas[current_page].1,
    );
    let _ = reply_handle
        .edit(
            ctx,
            poise::CreateReply::default()
                .content(last_content)
                .components(vec![disabled_buttons]),
        )
        .await;

    Ok(())
}

fn format_fixture_page(
    t: &Tournament,
    fecha: i32,
    page_idx: usize,
    total_pages: usize,
    matches: &[Match],
) -> String {
    let mut out = format!(
        "**{}** (ID {}, {}) — **Fecha {}** ({}/{})\n\n",
        t.name,
        t.tournament_id,
        if t.oficial { "oficial" } else { "amistoso" },
        fecha,
        page_idx + 1,
        total_pages
    );
    for m in matches {
        let score = match (m.home_goals, m.away_goals) {
            (Some(h), Some(a)) => format!("{h} - {a}"),
            _ => "? - ?".to_string(),
        };
        out.push_str(&format!("<@{}> {} <@{}>\n", m.home_id, score, m.away_id));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_fixture_page() {
        let t = Tournament {
            tournament_id: 42,
            guild_id: 1,
            name: "Torneo Apertura".into(),
            oficial: true,
            finished: false,
            champion_id: None,
        };
        let matches = vec![
            Match {
                fecha: 1,
                home_id: 101,
                away_id: 102,
                home_goals: Some(2),
                away_goals: Some(1),
            },
            Match {
                fecha: 1,
                home_id: 103,
                away_id: 104,
                home_goals: None,
                away_goals: None,
            },
        ];

        let page = format_fixture_page(&t, 1, 0, 3, &matches);
        assert!(page.contains("**Torneo Apertura** (ID 42, oficial) — **Fecha 1** (1/3)"));
        assert!(page.contains("<@101> 2 - 1 <@102>"));
        assert!(page.contains("<@103> ? - ? <@104>"));
    }
}
