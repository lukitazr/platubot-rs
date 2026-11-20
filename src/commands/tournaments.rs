use crate::{Context, Error, serenity};
use crate::database::repositories::tournaments as repo;

fn guild(ctx: Context<'_>) -> Result<i64, Error> {
    Ok(i64::try_from(ctx.guild_id().ok_or("Este comando solo se puede usar en un servidor")?.get())?)
}
fn uid(user: &serenity::User) -> Result<i64, Error> { Ok(i64::try_from(user.id.get())?) }

async fn autocomplete_torneo(ctx: Context<'_>, partial: &str) -> serenity::CreateAutocompleteResponse {
    let choices = match guild(ctx) {
        Ok(guild_id) => match repo::search(&ctx.data().pool, guild_id, partial, 25).await {
            Ok(tournaments) => tournaments.into_iter().map(|(id, name)| {
                let name: String = name.chars().take(70).collect();
                serenity::AutocompleteChoice::new(format!("{name} (ID: {id})"), id.to_string())
            }).collect(),
            Err(error) => { eprintln!("Error al buscar torneos: {error}"); Vec::new() }
        },
        Err(_) => Vec::new(),
    };
    serenity::CreateAutocompleteResponse::new().set_choices(choices)
}

async fn resolver_torneo(ctx: Context<'_>, input: &str) -> Result<Option<i64>, Error> {
    let input = input.trim();
    let guild_id = guild(ctx)?;
    if let Ok(id) = input.parse::<i64>() {
        if repo::get(&ctx.data().pool, guild_id, id).await?.is_some() { return Ok(Some(id)); }
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
    ctx.say("Torneo no encontrado en este servidor. Usa un ID o selecciona una sugerencia.").await?;
    Ok(None)
}

#[poise::command(slash_command, guild_only, subcommands("crear", "inscribir", "programar", "resultado", "wo", "ver", "finalizar"))]
pub async fn torneo(_: Context<'_>) -> Result<(), Error> { Ok(()) }

#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
pub async fn crear(ctx: Context<'_>, nombre: String, oficial: bool) -> Result<(), Error> {
    let id = repo::create(&ctx.data().pool, guild(ctx)?, &nombre, oficial).await?;
    ctx.say(format!("Torneo **{nombre}** creado con ID `{id}`.")).await?; Ok(())
}

#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
pub async fn inscribir(ctx: Context<'_>, torneo: i64, usuario: serenity::User) -> Result<(), Error> {
    let name = usuario.global_name.as_deref().unwrap_or(&usuario.name);
    let added = repo::join(&ctx.data().pool, guild(ctx)?, torneo, uid(&usuario)?, name).await?;
    ctx.say(if added { "Participante inscrito." } else { "El usuario ya está inscrito." }).await?; Ok(())
}

#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
pub async fn programar(ctx: Context<'_>, torneo: i64, fecha: i32, local: serenity::User, visitante: serenity::User) -> Result<(), Error> {
    if fecha <= 0 || local.id == visitante.id { ctx.say("La fecha debe ser positiva y los jugadores deben ser distintos.").await?; return Ok(()); }
    let id = repo::schedule(&ctx.data().pool, guild(ctx)?, torneo, fecha, uid(&local)?, uid(&visitante)?).await?;
    ctx.say(format!("Enfrentamiento programado con ID `{id}`.")).await?; Ok(())
}

#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
pub async fn resultado(ctx: Context<'_>, torneo: i64, partido: i64, goles_local: i64, goles_visitante: i64) -> Result<(), Error> {
    if goles_local < 0 || goles_visitante < 0 { ctx.say("Los goles no pueden ser negativos.").await?; return Ok(()); }
    let saved = repo::result(&ctx.data().pool, guild(ctx)?, torneo, partido, goles_local, goles_visitante, None).await?;
    ctx.say(if saved { "Resultado registrado y estadísticas actualizadas." } else { "Partido inexistente o con resultado ya registrado." }).await?; Ok(())
}

#[poise::command(slash_command)]
pub async fn ver(ctx: Context<'_>, torneo: i64) -> Result<(), Error> {
    let pool = &ctx.data().pool;
    let Some(t) = repo::get(pool, guild(ctx)?, torneo).await? else { ctx.say("Torneo no encontrado.").await?; return Ok(()); };
    let participants = repo::participants(pool, torneo).await?;
    let matches = repo::matches(pool, torneo).await?;
    let mut out = format!("**{}** (ID {}, {}) — {}\n", t.name, t.tournament_id, if t.oficial { "oficial" } else { "amistoso" }, if t.finished { "finalizado" } else { "en curso" });
    if let Some(champion) = t.champion_id { out.push_str(&format!("Campeón: <@{champion}>\n")); }
    out.push_str("\n**Clasificación** (3 puntos por victoria, 1 por empate)\n");
    for p in participants { out.push_str(&format!("<@{}> — {} pts | {}V {}E {}D | {}:{}\n", p.user_id, p.pg * 3 + p.pe, p.pg, p.pe, p.pp, p.gf, p.gc)); }
    out.push_str("\n**Enfrentamientos**\n");
    for m in matches { out.push_str(&format!("#{} · Fecha {}: <@{}> {} - {} <@{}>\n", m.match_id, m.fecha, m.home_id, m.home_goals.map_or("?".into(), |n| n.to_string()), m.away_goals.map_or("?".into(), |n| n.to_string()), m.away_id)); }
    // Discord messages are limited to 2,000 characters; split on line boundaries.
    let mut chunk = String::new();
    for line in out.lines() {
        if chunk.len() + line.len() + 1 > 1900 && !chunk.is_empty() { ctx.say(&chunk).await?; chunk.clear(); }
        if line.len() > 1900 { continue; }
        chunk.push_str(line); chunk.push('\n');
    }
    if !chunk.is_empty() { ctx.say(chunk).await?; }
    Ok(())
}

#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
pub async fn finalizar(ctx: Context<'_>, torneo: i64, campeon: serenity::User) -> Result<(), Error> {
    let done = repo::finish(&ctx.data().pool, guild(ctx)?, torneo, uid(&campeon)?).await?;
    ctx.say(if done { "Torneo finalizado y título añadido al campeón." } else { "El campeón debe estar inscrito y no puede haber partidos pendientes." }).await?; Ok(())
}

#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
pub async fn wo(ctx: Context<'_>, torneo: i64, partido: i64, ausente: serenity::User, goles_local: i64, goles_visitante: i64) -> Result<(), Error> {
    if goles_local < 0 || goles_visitante < 0 { ctx.say("Los goles no pueden ser negativos.").await?; return Ok(()); }
    let saved = repo::result(&ctx.data().pool, guild(ctx)?, torneo, partido, goles_local, goles_visitante, Some(uid(&ausente)?)).await?;
    ctx.say(if saved { "Resultado por WO registrado y estadísticas actualizadas." } else { "El partido no está pendiente o el ausente no es el jugador derrotado." }).await?;
    Ok(())
}
