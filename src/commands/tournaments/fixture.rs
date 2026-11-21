mod imagen;

use super::repo;
use super::shared::{autocomplete_torneo, descargar_avatares, guild, resolver_torneo, uid};
use crate::database::models::tournaments::{Match, Tournament};
use crate::{Context, Error, serenity};
use bot_command_macros::bot_command;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::Arc,
    time::Duration,
};
use tokio::time::Instant;

struct Fixture {
    tournament: Tournament,
    dates: Vec<(i32, Vec<Match>)>,
    names: HashMap<i64, String>,
    avatars: HashMap<i64, Vec<u8>>,
    player: Option<i64>,
}

/// Muestra los enfrentamientos de un torneo, organizados por fecha.
#[bot_command(slash_command, guild_only)]
pub async fn fixture(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_torneo"]
    #[description = "Nombre o ID del torneo"]
    torneo: String,
    #[description = "Fecha inicial del fixture general"] fecha: Option<i32>,
    #[description = "Mostrar solo los partidos de este jugador"] jugador: Option<serenity::User>,
) -> Result<(), Error> {
    ctx.defer().await?;
    let Some(torneo) = resolver_torneo(ctx, &torneo).await? else {
        return Ok(());
    };
    let pool = &ctx.data().pool;
    let Some(tournament) = repo::get(pool, guild(ctx)?, torneo).await? else {
        ctx.say("Torneo no encontrado.").await?;
        return Ok(());
    };
    let matches = repo::matches(pool, torneo).await?;
    if matches.is_empty() {
        ctx.say("El torneo no tiene enfrentamientos programados.")
            .await?;
        return Ok(());
    }
    let player = jugador.as_ref().map(uid).transpose()?;
    let dates = fechas(matches, player);
    if dates.is_empty() {
        ctx.say("El jugador no tiene enfrentamientos programados en este torneo.")
            .await?;
        return Ok(());
    }
    let mut current = match fecha.filter(|_| player.is_none()) {
        Some(date) => match dates.binary_search_by_key(&date, |(f, _)| *f) {
            Ok(index) => index,
            Err(_) => {
                ctx.say(format!(
                    "No hay partidos programados en la fecha {date} de este torneo."
                ))
                .await?;
                return Ok(());
            }
        },
        None if player.is_none() => dates
            .iter()
            .position(|(_, games)| {
                games
                    .iter()
                    .any(|game| game.home_goals.zip(game.away_goals).is_none())
            })
            .unwrap_or(0),
        None => 0,
    };
    let ids: Vec<_> = dates
        .iter()
        .flat_map(|(_, matches)| matches)
        .flat_map(|game| [game.home_id, game.away_id])
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let mut names: HashMap<_, _> = repo::participants(pool, torneo)
        .await?
        .into_iter()
        .map(|p| (p.user_id, p.username))
        .collect();
    if let (Some(id), Some(user)) = (player, jugador) {
        names.entry(id).or_insert(user.name);
    }
    let avatars = descargar_avatares(ctx, &ids).await;
    let total = dates.len();
    let fixture = Arc::new(Fixture {
        tournament,
        dates,
        names,
        avatars,
        player,
    });
    if let Some(player) = player {
        let png =
            tokio::task::spawn_blocking(move || imagen::render_player(&fixture, player)).await??;
        let image = serenity::CreateAttachment::bytes(
            png,
            format!("fixture-{torneo}-jugador-{player}.png"),
        )
        .description("Todas las fechas del jugador, en orden cronológico por columna.");
        ctx.send(poise::CreateReply::default().attachment(image))
            .await?;
        return Ok(());
    }
    let image = attachment(&fixture, current).await?;
    if total == 1 {
        ctx.send(poise::CreateReply::default().attachment(image))
            .await?;
        return Ok(());
    }

    let prefix = ctx.id();
    let prev = format!("{prefix}_fixture_prev");
    let next = format!("{prefix}_fixture_next");
    let buttons = |page: usize, disabled: bool| {
        vec![serenity::CreateActionRow::Buttons(vec![
            serenity::CreateButton::new(&prev)
                .emoji('◀')
                .label("Anterior")
                .disabled(disabled),
            serenity::CreateButton::new(format!("{prefix}_fixture_page"))
                .label(format!("{}/{total}", page + 1))
                .disabled(true),
            serenity::CreateButton::new(&next)
                .emoji('▶')
                .label("Siguiente")
                .disabled(disabled),
        ])]
    };
    let reply = ctx
        .send(
            poise::CreateReply::default()
                .attachment(image)
                .components(buttons(current, false)),
        )
        .await?;
    let deadline = Instant::now() + Duration::from_secs(600);
    while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
        let prev_id = prev.clone();
        let next_id = next.clone();
        let Some(press) = serenity::collector::ComponentInteractionCollector::new(ctx)
            .filter(move |press| press.data.custom_id == prev_id || press.data.custom_id == next_id)
            .timeout(remaining)
            .await
        else {
            break;
        };
        press.defer(ctx.serenity_context()).await?;
        let page = if press.data.custom_id == next {
            (current + 1) % total
        } else {
            current.checked_sub(1).unwrap_or(total - 1)
        };
        let image = attachment(&fixture, page).await?;
        press
            .edit_response(
                ctx.serenity_context(),
                serenity::EditInteractionResponse::new()
                    .attachments(serenity::EditAttachments::new().add(image))
                    .components(buttons(page, false)),
            )
            .await?;
        current = page;
    }
    let _ = reply
        .edit(
            ctx,
            poise::CreateReply::default().components(buttons(current, true)),
        )
        .await;
    Ok(())
}

fn fechas(matches: Vec<Match>, player: Option<i64>) -> Vec<(i32, Vec<Match>)> {
    let mut dates = BTreeMap::<i32, Vec<Match>>::new();
    for game in matches {
        if player.is_none_or(|id| id == game.home_id || id == game.away_id) {
            dates.entry(game.fecha).or_default().push(game);
        }
    }
    dates.into_iter().collect()
}

async fn attachment(
    fixture: &Arc<Fixture>,
    page: usize,
) -> Result<serenity::CreateAttachment, Error> {
    let fixture = Arc::clone(fixture);
    tokio::task::spawn_blocking(move || {
        let date = fixture.dates[page].0;
        let png = imagen::render(&fixture, page)?;
        Ok(serenity::CreateAttachment::bytes(
            png,
            format!(
                "fixture-{}-fecha-{date}.png",
                fixture.tournament.tournament_id
            ),
        )
        .description(format!(
            "Fixture de {}. Fecha {date}. Página {} de {}.",
            fixture
                .tournament
                .name
                .chars()
                .take(200)
                .collect::<String>(),
            page + 1,
            fixture.dates.len()
        )))
    })
    .await?
}
