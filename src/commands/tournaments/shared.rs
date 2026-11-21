use super::repo;
use crate::{Context, Error, serenity};

pub(super) fn guild(ctx: Context<'_>) -> Result<i64, Error> {
    Ok(i64::try_from(
        ctx.guild_id()
            .ok_or("Este comando solo se puede usar en un servidor")?
            .get(),
    )?)
}
pub(super) fn uid(user: &serenity::User) -> Result<i64, Error> {
    Ok(i64::try_from(user.id.get())?)
}

pub(super) async fn autocomplete_torneo(
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

pub(super) async fn resolver_torneo(ctx: Context<'_>, input: &str) -> Result<Option<i64>, Error> {
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

pub(super) async fn autocomplete_participante(
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

pub(super) async fn resolver_participante(
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

pub(super) fn describir_resultado(outcome: repo::MatchResult, wo: bool) -> String {
    match outcome {
        repo::MatchResult::Saved => if wo { "Resultado por WO registrado y estadísticas actualizadas." } else { "Resultado registrado y estadísticas actualizadas." }.into(),
        repo::MatchResult::AlreadyPlayed => "Este partido ya tiene un resultado registrado.".into(),
        repo::MatchResult::InvalidWalkover => "El ausente debe ser el jugador derrotado de este partido.".into(),
        repo::MatchResult::Ambiguous => "Hay partidos duplicados entre estos jugadores en esta fecha; no se ha registrado el resultado.".into(),
        repo::MatchResult::NotFound => "No existe un partido entre estos participantes en el torneo.".into(),
        repo::MatchResult::Closest { fecha, home, away, played } => format!("No hay un partido entre estos participantes en la fecha indicada. La coincidencia más cercana es la fecha {fecha}: <@{home}> (local) contra <@{away}> (visitante){}.", if played { ", con resultado ya registrado" } else { "" }),
    }
}

pub(super) async fn descargar_avatares(
    ctx: Context<'_>,
    ids: &[i64],
) -> std::collections::HashMap<i64, Vec<u8>> {
    let mut avatars = std::collections::HashMap::new();
    for batch in ids.chunks(8) {
        let mut downloads = tokio::task::JoinSet::new();
        for &id in batch {
            let http = ctx.serenity_context().http.clone();
            downloads.spawn(async move {
                let png =
                    tokio::time::timeout(std::time::Duration::from_secs(5), avatar_png(&http, id))
                        .await
                        .ok()
                        .flatten();
                (id, png)
            });
        }
        while let Some(result) = downloads.join_next().await {
            if let Ok((id, Some(png))) = result {
                avatars.insert(id, png);
            }
        }
    }
    avatars
}

async fn avatar_png(http: &serenity::Http, id: i64) -> Option<Vec<u8>> {
    let user = serenity::UserId::new(u64::try_from(id).ok()?)
        .to_user(http)
        .await
        .ok()?;
    let url = match user.avatar {
        Some(hash) => format!(
            "https://cdn.discordapp.com/avatars/{}/{hash}.png?size=64",
            user.id
        ),
        None => user.default_avatar_url(),
    };
    serenity::CreateAttachment::url(http, &url)
        .await
        .ok()
        .map(|file| file.data)
}
