use crate::database::repositories::tournaments as repo;
use crate::{Context, Error, serenity};

fn guild(ctx: Context<'_>) -> Result<i64, Error> {
    Ok(i64::try_from(
        ctx.guild_id()
            .ok_or("Este comando solo se puede usar en un servidor")?
            .get(),
    )?)
}
fn uid(user: &serenity::User) -> Result<i64, Error> {
    Ok(i64::try_from(user.id.get())?)
}

async fn autocomplete_torneo(
    ctx: Context<'_>,
    partial: &str,
) -> serenity::CreateAutocompleteResponse {
    let choices = match guild(ctx) {
        Ok(guild_id) => match repo::search(&ctx.data().pool, guild_id, partial, 25).await {
            Ok(tournaments) => tournaments
                .into_iter()
                .map(|(id, name)| {
                    let name: String = name.chars().take(70).collect();
                    serenity::AutocompleteChoice::new(format!("{name} (ID: {id})"), id.to_string())
                })
                .collect(),
            Err(error) => {
                eprintln!("Error al buscar torneos: {error}");
                Vec::new()
            }
        },
        Err(_) => Vec::new(),
    };
    serenity::CreateAutocompleteResponse::new().set_choices(choices)
}

async fn resolver_torneo(ctx: Context<'_>, input: &str) -> Result<Option<i64>, Error> {
    let input = input.trim();
    let guild_id = guild(ctx)?;
    if let Ok(id) = input.parse::<i64>() {
        if repo::get(&ctx.data().pool, guild_id, id).await?.is_some() {
            return Ok(Some(id));
        }
    } else if !input.is_empty() {
        let matches = repo::search_name(&ctx.data().pool, guild_id, input).await?;
        match matches.as_slice() {
            [(id, _)] => return Ok(Some(*id)),
            [_, _, ..] => {
                ctx.say("Hay varios torneos con ese prefijo. Selecciona uno de las sugerencias o indica su ID.").await?;
                return Ok(None);
            }
            _ => {}
        }
    }
    ctx.say("Torneo no encontrado en este servidor. Usa un ID o selecciona una sugerencia.")
        .await?;
    Ok(None)
}

async fn autocomplete_participante(
    ctx: Context<'_>,
    partial: &str,
) -> serenity::CreateAutocompleteResponse {
    let choices = async {
        let poise::Context::Application(app) = ctx else {
            return Ok::<_, sqlx::Error>(Vec::new());
        };
        let Some(input) = app
            .args
            .iter()
            .find_map(|arg| match (&*arg.name, &arg.value) {
                ("torneo", serenity::ResolvedValue::String(value)) => Some(*value),
                _ => None,
            })
        else {
            return Ok(Vec::new());
        };
        let Ok(guild_id) = guild(ctx) else {
            return Ok(Vec::new());
        };
        let input = input.trim();
        let tournament = if let Ok(id) = input.parse::<i64>() {
            repo::get(&ctx.data().pool, guild_id, id)
                .await?
                .map(|t| t.tournament_id)
        } else if !input.is_empty() {
            let found = repo::search_name(&ctx.data().pool, guild_id, input).await?;
            match found.as_slice() {
                [(id, _)] => Some(*id),
                _ => None,
            }
        } else {
            None
        };
        let Some(tournament) = tournament else {
            return Ok(Vec::new());
        };
        let participants =
            repo::search_participants(&ctx.data().pool, guild_id, tournament, partial).await?;
        Ok(participants
            .into_iter()
            .map(|(id, name)| {
                let name: String = name.chars().take(70).collect();
                serenity::AutocompleteChoice::new(format!("{name} (ID: {id})"), id.to_string())
            })
            .collect())
    }
    .await;
    match choices {
        Ok(choices) => serenity::CreateAutocompleteResponse::new().set_choices(choices),
        Err(error) => {
            eprintln!("Error al buscar participantes: {error}");
            serenity::CreateAutocompleteResponse::new()
        }
    }
}

async fn resolver_participante(
    ctx: Context<'_>,
    torneo: i64,
    input: &str,
) -> Result<Option<i64>, Error> {
    if let Ok(id) = input.trim().parse::<i64>() {
        if repo::is_participant(&ctx.data().pool, guild(ctx)?, torneo, id).await? {
            return Ok(Some(id));
        }
    }
    ctx.say("Selecciona un participante inscrito en este torneo de las sugerencias, o indica su ID completo.").await?;
    Ok(None)
}

#[poise::command(
    slash_command,
    guild_only,
    subcommands(
        "crear",
        "inscribir",
        "programar",
        "sortear",
        "resultado",
        "wo",
        "ver",
        "finalizar",
        "fixture",
        "tabla"
    )
)]
pub async fn torneo(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
pub async fn crear(ctx: Context<'_>, nombre: String, oficial: bool) -> Result<(), Error> {
    let id = repo::create(&ctx.data().pool, guild(ctx)?, &nombre, oficial).await?;
    ctx.say(format!("Torneo **{nombre}** creado con ID `{id}`."))
        .await?;
    Ok(())
}

/// Genera el calendario de todos contra todos.
#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
pub async fn sortear(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_torneo"] torneo: String,
    #[description = "Cantidad de rondas (por defecto 1)"] rondas: Option<u32>,
) -> Result<(), Error> {
    let Some(torneo) = resolver_torneo(ctx, &torneo).await? else {
        return Ok(());
    };
    let rondas = rondas.unwrap_or(1);
    if !(1..=20).contains(&rondas) {
        ctx.say("La cantidad de rondas debe estar entre 1 y 20.")
            .await?;
        return Ok(());
    }
    ctx.defer().await?;
    let response = match repo::draw(&ctx.data().pool, guild(ctx)?, torneo, rondas as usize).await {
        Ok(repo::DrawResult::Created { dates, matches }) => format!("Sorteo creado: {dates} fechas y {matches} partidos en {rondas} ronda(s)."),
        Ok(repo::DrawResult::ExistingMatches) => "El torneo ya tiene partidos; no se ha modificado el calendario.".into(),
        Ok(repo::DrawResult::TooFewParticipants) => "Se necesitan al menos dos participantes inscritos.".into(),
        Ok(repo::DrawResult::UnequalHomeAway) => "Con un número par de participantes se necesita un número par de rondas para igualar los partidos de local y visitante.".into(),
        Ok(repo::DrawResult::UnsupportedSize) => "El sorteo admite hasta 64 participantes y entre 1 y 20 rondas.".into(),
        Ok(repo::DrawResult::NoSchedule) => "No se encontró un calendario válido; prueba nuevamente.".into(),
        Err(sqlx::Error::RowNotFound) => "El torneo ya no existe o está finalizado.".into(),
        Err(error) => return Err(error.into()),
    };
    ctx.say(response).await?;
    Ok(())
}

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

#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
pub async fn resultado(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_torneo"] torneo: String,
    fecha: i32,
    #[autocomplete = "autocomplete_participante"] jugador1: String,
    #[autocomplete = "autocomplete_participante"] jugador2: String,
    goles_jugador1: i64,
    goles_jugador2: i64,
) -> Result<(), Error> {
    let Some(torneo) = resolver_torneo(ctx, &torneo).await? else {
        return Ok(());
    };
    let Some(jugador1) = resolver_participante(ctx, torneo, &jugador1).await? else {
        return Ok(());
    };
    let Some(jugador2) = resolver_participante(ctx, torneo, &jugador2).await? else {
        return Ok(());
    };
    if fecha <= 0 || jugador1 == jugador2 || goles_jugador1 < 0 || goles_jugador2 < 0 {
        ctx.say("La fecha debe ser positiva, los jugadores distintos y los goles no negativos.")
            .await?;
        return Ok(());
    }
    let outcome = repo::result(
        &ctx.data().pool,
        guild(ctx)?,
        torneo,
        fecha,
        jugador1,
        jugador2,
        goles_jugador1,
        goles_jugador2,
        None,
    )
    .await?;
    ctx.say(describir_resultado(outcome, false)).await?;
    Ok(())
}

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
            "<@{}> — {} pts | {}V {}E {}D | {}:{}\n",
            p.user_id,
            p.pg * 3 + p.pe,
            p.pg,
            p.pe,
            p.pp,
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

#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
pub async fn wo(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_torneo"] torneo: String,
    fecha: i32,
    #[autocomplete = "autocomplete_participante"] jugador1: String,
    #[autocomplete = "autocomplete_participante"] jugador2: String,
    #[autocomplete = "autocomplete_participante"] ausente: String,
    goles_jugador1: i64,
    goles_jugador2: i64,
) -> Result<(), Error> {
    let Some(torneo) = resolver_torneo(ctx, &torneo).await? else {
        return Ok(());
    };
    let Some(ausente) = resolver_participante(ctx, torneo, &ausente).await? else {
        return Ok(());
    };
    let Some(jugador1) = resolver_participante(ctx, torneo, &jugador1).await? else {
        return Ok(());
    };
    let Some(jugador2) = resolver_participante(ctx, torneo, &jugador2).await? else {
        return Ok(());
    };
    if fecha <= 0 || jugador1 == jugador2 || goles_jugador1 < 0 || goles_jugador2 < 0 {
        ctx.say("La fecha debe ser positiva, los jugadores distintos y los goles no negativos.")
            .await?;
        return Ok(());
    }
    let outcome = repo::result(
        &ctx.data().pool,
        guild(ctx)?,
        torneo,
        fecha,
        jugador1,
        jugador2,
        goles_jugador1,
        goles_jugador2,
        Some(ausente),
    )
    .await?;
    ctx.say(describir_resultado(outcome, true)).await?;
    Ok(())
}

fn format_fixture_page(
    t: &crate::database::models::tournaments::Tournament,
    fecha: i32,
    page_idx: usize,
    total_pages: usize,
    matches: &[crate::database::models::tournaments::Match],
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

/// Muestra los enfrentamientos del torneo paginados por fecha.
#[poise::command(slash_command, guild_only)]
pub async fn fixture(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_torneo"]
    #[description = "Nombre o ID del torneo"]
    torneo: String,
    #[description = "Fecha inicial a consultar (opcional)"]
    fecha: Option<i32>,
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
        ctx.say("El torneo no tiene enfrentamientos programados.").await?;
        return Ok(());
    }

    let mut fechas_map =
        std::collections::BTreeMap::<i32, Vec<crate::database::models::tournaments::Match>>::new();
    for m in matches {
        fechas_map.entry(m.fecha).or_default().push(m);
    }
    let fechas: Vec<(i32, Vec<crate::database::models::tournaments::Match>)> =
        fechas_map.into_iter().collect();
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
        .timeout(std::time::Duration::from_secs(600))
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

/// Muestra la tabla de posiciones y clasificación del torneo.
#[poise::command(slash_command, guild_only)]
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
        ctx.say("El torneo no tiene participantes inscritos.").await?;
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

fn format_standings(
    t: &crate::database::models::tournaments::Tournament,
    participants: &[crate::database::models::tournaments::Participant],
) -> String {
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

fn describir_resultado(outcome: repo::MatchResult, wo: bool) -> String {
    match outcome {
        repo::MatchResult::Saved => if wo { "Resultado por WO registrado y estadísticas actualizadas." } else { "Resultado registrado y estadísticas actualizadas." }.into(),
        repo::MatchResult::AlreadyPlayed => "Este partido ya tiene un resultado registrado.".into(),
        repo::MatchResult::InvalidWalkover => "El ausente debe ser el jugador derrotado de este partido.".into(),
        repo::MatchResult::Ambiguous => "Hay partidos duplicados entre estos jugadores en esta fecha; no se ha registrado el resultado.".into(),
        repo::MatchResult::NotFound => "No existe un partido entre estos participantes en el torneo.".into(),
        repo::MatchResult::Closest { fecha, home, away, played } => format!("No hay un partido entre estos participantes en la fecha indicada. La coincidencia más cercana es la fecha {fecha}: <@{home}> (local) contra <@{away}> (visitante){}.", if played { ", con resultado ya registrado" } else { "" }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::models::tournaments::{Match, Participant, Tournament};

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
