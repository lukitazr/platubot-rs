use super::repo;
use super::shared::{autocomplete_torneo, guild, resolver_torneo};
use crate::{Context, Error, serenity};

async fn autocomplete_partido(
    ctx: Context<'_>,
    partial: &str,
) -> serenity::CreateAutocompleteResponse {
    let choices = async {
        let poise::Context::Application(app) = ctx else { return Ok::<_, sqlx::Error>(Vec::new()); };
        let Some(input) = app.args.iter().find_map(|arg| match (&*arg.name, &arg.value) {
            ("torneo", serenity::ResolvedValue::String(value)) => Some(value.trim()),
            _ => None,
        }) else { return Ok(Vec::new()); };
        if input.is_empty() {
            return Ok(Vec::new());
        }
        let Ok(guild) = guild(ctx) else { return Ok(Vec::new()); };
        let tournament = if let Ok(id) = input.parse::<i64>() {
            repo::get(&ctx.data().pool, guild, id).await?.map(|t| t.tournament_id)
        } else {
            match repo::search_name(&ctx.data().pool, guild, input).await?.as_slice() {
                [(id, _)] => Some(*id), _ => None,
            }
        };
        let Some(tournament) = tournament else { return Ok(Vec::new()); };
        let games: Vec<(i64, i32, String, String)> = sqlx::query_as(
            "SELECT m.match_id, m.fecha, h.username, a.username FROM tournament_matches m JOIN tournaments t ON t.tournament_id=m.tournament_id JOIN users h ON h.id=m.home_id JOIN users a ON a.id=m.away_id WHERE t.guild_id=$1 AND t.tournament_id=$2 AND NOT t.finished AND m.home_goals IS NULL AND ($3='' OR position(lower($3) in lower(h.username))>0 OR position(lower($3) in lower(a.username))>0 OR position($3 in m.match_id::text)>0 OR position($3 in m.fecha::text)>0) ORDER BY m.fecha, m.match_id LIMIT 25"
        ).bind(guild).bind(tournament).bind(partial.trim()).fetch_all(&ctx.data().pool).await?;
        Ok(games.into_iter().map(|(id, fecha, home, away)| {
            let label = format!("Fecha {fecha}: {} vs {} (ID {id})", home.chars().take(25).collect::<String>(), away.chars().take(25).collect::<String>());
            serenity::AutocompleteChoice::new(label, id.to_string())
        }).collect())
    }.await;
    match choices {
        Ok(choices) => serenity::CreateAutocompleteResponse::new().set_choices(choices),
        Err(error) => {
            eprintln!("Error al buscar partidos pendientes: {error}");
            serenity::CreateAutocompleteResponse::new()
        }
    }
}

/// Selecciona un partido pendiente y registra su resultado.
#[poise::command(slash_command, required_permissions = "MANAGE_GUILD")]
pub async fn resultado(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_torneo"] torneo: String,
    #[autocomplete = "autocomplete_partido"] partido: String,
) -> Result<(), Error> {
    let Some(torneo) = resolver_torneo(ctx, &torneo).await? else {
        return Ok(());
    };
    let Ok(partido) = partido.trim().parse::<i64>() else {
        ctx.say("Selecciona un partido pendiente de las sugerencias.")
            .await?;
        return Ok(());
    };
    let players: Option<(String, String)> = sqlx::query_as("SELECT h.username,a.username FROM tournament_matches m JOIN tournaments t ON t.tournament_id=m.tournament_id JOIN users h ON h.id=m.home_id JOIN users a ON a.id=m.away_id WHERE t.guild_id=$1 AND t.tournament_id=$2 AND NOT t.finished AND m.match_id=$3 AND m.home_goals IS NULL")
        .bind(guild(ctx)?).bind(torneo).bind(partido).fetch_optional(&ctx.data().pool).await?;
    let Some((home, away)) = players else {
        ctx.say("El partido no existe en este torneo o ya tiene resultado.")
            .await?;
        return Ok(());
    };
    let poise::Context::Application(app) = ctx else {
        return Ok(());
    };
    app.interaction
        .create_response(
            ctx.serenity_context(),
            crate::events::submissions::score_modal(
                format!("result:{torneo}:{partido}"),
                &home,
                &away,
            ),
        )
        .await?;
    Ok(())
}
