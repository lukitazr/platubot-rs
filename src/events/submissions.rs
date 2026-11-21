use crate::{Error, serenity as s};
use s::Mentionable;
use sqlx::PgPool;

fn buttons(id: i64, review: bool) -> Vec<s::CreateActionRow> {
    let (a, b) = if review {
        ("Validar resultado", "Denegar captura")
    } else {
        ("Elegir partido", "Ignorar envío")
    };
    vec![s::CreateActionRow::Buttons(vec![
        s::CreateButton::new(format!(
            "sub:{}:{id}",
            if review { "validate" } else { "choose" }
        ))
        .label(a)
        .style(s::ButtonStyle::Primary),
        s::CreateButton::new(format!(
            "sub:{}:{id}",
            if review { "deny" } else { "ignore" }
        ))
        .label(b)
        .style(s::ButtonStyle::Danger),
    ])]
}
pub(crate) fn text_response(text: impl Into<String>) -> s::CreateInteractionResponse {
    s::CreateInteractionResponse::Message(
        s::CreateInteractionResponseMessage::new()
            .content(text)
            .ephemeral(true),
    )
}
pub(crate) fn admin(member: &Option<s::Member>) -> bool {
    member
        .as_ref()
        .and_then(|m| m.permissions)
        .is_some_and(|p| p.manage_guild() || p.administrator())
}
fn field(label: String, id: &str) -> s::CreateActionRow {
    s::CreateActionRow::InputText(
        s::CreateInputText::new(s::InputTextStyle::Short, label, id)
            .required(true)
            .max_length(20),
    )
}
pub(crate) fn value<'a>(m: &'a s::ModalInteraction, id: &str) -> Option<&'a str> {
    m.data
        .components
        .iter()
        .flat_map(|r| &r.components)
        .find_map(|c| match c {
            s::ActionRowComponent::InputText(t) if t.custom_id == id => t.value.as_deref(),
            _ => None,
        })
}

pub(crate) fn score_modal(
    custom_id: String,
    home: &str,
    away: &str,
) -> s::CreateInteractionResponse {
    s::CreateInteractionResponse::Modal(
        s::CreateModal::new(custom_id, "Validar resultado").components(vec![
            field(
                format!(
                    "Goles {} (local)",
                    home.chars().take(25).collect::<String>()
                ),
                "home",
            ),
            field(
                format!(
                    "Goles {} (visitante)",
                    away.chars().take(20).collect::<String>()
                ),
                "away",
            ),
        ]),
    )
}

async fn selection_menu(
    pool: &PgPool,
    id: i64,
    page: i64,
) -> Result<Vec<s::CreateActionRow>, Error> {
    let games: Vec<(i64,i32,String,String)> = sqlx::query_as("SELECT m.match_id,m.fecha,h.username,a.username FROM match_submissions s JOIN tournament_matches m ON m.tournament_id=s.tournament_id JOIN users h ON h.id=m.home_id JOIN users a ON a.id=m.away_id JOIN tournaments t ON t.tournament_id=s.tournament_id WHERE s.submission_id=$1 AND NOT t.finished AND m.home_goals IS NULL AND (m.home_id=s.author_id OR m.away_id=s.author_id) ORDER BY m.fecha,m.match_id LIMIT 26 OFFSET $2").bind(id).bind(page * 25).fetch_all(pool).await?;
    let mut rows = Vec::new();
    if !games.is_empty() {
        let options = games
            .iter()
            .take(25)
            .map(|(id, date, home, away)| {
                s::CreateSelectMenuOption::new(
                    format!(
                        "Fecha {date}: {} vs {}",
                        home.chars().take(30).collect::<String>(),
                        away.chars().take(30).collect::<String>()
                    ),
                    id.to_string(),
                )
            })
            .collect();
        rows.push(s::CreateActionRow::SelectMenu(
            s::CreateSelectMenu::new(
                format!("sub:pick:{id}"),
                s::CreateSelectMenuKind::String { options },
            )
            .placeholder("Selecciona el partido"),
        ));
    }
    let mut controls = Vec::new();
    if page > 0 {
        controls
            .push(s::CreateButton::new(format!("sub:page:{id}:{}", page - 1)).label("Anterior"));
    }
    if games.len() > 25 {
        controls
            .push(s::CreateButton::new(format!("sub:page:{id}:{}", page + 1)).label("Siguiente"));
    }
    controls.push(
        s::CreateButton::new(format!("sub:ignore:{id}"))
            .label("Ignorar envío")
            .style(s::ButtonStyle::Danger),
    );
    rows.push(s::CreateActionRow::Buttons(controls));
    Ok(rows)
}

pub async fn message(ctx: &s::Context, msg: &s::Message, pool: &PgPool) -> Result<(), Error> {
    if msg.author.bot
        || msg.guild_id.is_none()
        || !msg.attachments.iter().any(|a| {
            a.content_type
                .as_deref()
                .is_some_and(|t| t.starts_with("image/"))
        })
    {
        return Ok(());
    }
    let guild = i64::try_from(msg.guild_id.unwrap().get())?;
    let author = i64::try_from(msg.author.id.get())?;
    let channel = i64::try_from(msg.channel_id.get())?;
    let tournament: Option<i64> = sqlx::query_scalar("SELECT t.tournament_id FROM tournaments t JOIN tournament_participants p ON p.tournament_id=t.tournament_id WHERE t.guild_id=$1 AND t.submit_channel_id=$2 AND NOT t.finished AND p.user_id=$3") .bind(guild).bind(channel).bind(author).fetch_optional(pool).await?;
    let Some(tournament) = tournament else {
        return Ok(());
    };
    let id: i64 = sqlx::query_scalar("INSERT INTO match_submissions (tournament_id, author_id, source_channel_id, source_message_id) VALUES ($1,$2,$3,$4) RETURNING submission_id").bind(tournament).bind(author).bind(channel).bind(i64::try_from(msg.id.get())?).fetch_one(pool).await?;
    msg.channel_id
        .send_message(
            &ctx.http,
            s::CreateMessage::new()
                .content(format!(
                    "{}: elige el partido de estas imágenes o ignora el envío.",
                    msg.author.id.mention()
                ))
                .reference_message(msg)
                .components(selection_menu(pool, id, 0).await?),
        )
        .await?;
    Ok(())
}

pub async fn interaction(
    ctx: &s::Context,
    interaction: &s::Interaction,
    pool: &PgPool,
) -> Result<(), Error> {
    match interaction {
        s::Interaction::Component(c) if c.data.custom_id.starts_with("sub:") => {
            let parts: Vec<_> = c.data.custom_id.split(':').collect();
            let Some(id) = parts.get(2).and_then(|x| x.parse::<i64>().ok()) else {
                return Ok(());
            };
            let Some(guild) = c.guild_id else {
                return Ok(());
            };
            let row: Option<(i64,i64,String,Option<i64>,Option<i64>,Option<i64>)> = sqlx::query_as("SELECT s.author_id,s.tournament_id,s.status,s.match_id,s.review_channel_id,s.review_message_id FROM match_submissions s JOIN tournaments t ON t.tournament_id=s.tournament_id WHERE s.submission_id=$1 AND t.guild_id=$2").bind(id).bind(i64::try_from(guild.get())?).fetch_optional(pool).await?;
            let Some((author, _, status, game, review_channel, review_message)) = row else {
                c.create_response(ctx, text_response("Envío no encontrado."))
                    .await?;
                return Ok(());
            };
            let action = parts[1];
            if matches!(action, "pick" | "page" | "ignore")
                && (author != i64::try_from(c.user.id.get())? || status != "choosing")
            {
                c.create_response(
                    ctx,
                    text_response("Solo el autor puede gestionar un envío pendiente de selección."),
                )
                .await?;
                return Ok(());
            }
            if matches!(action, "validate" | "deny")
                && (!admin(&c.member)
                    || status != "pending"
                    || review_channel != Some(i64::try_from(c.channel_id.get())?)
                    || review_message != Some(i64::try_from(c.message.id.get())?))
            {
                c.create_response(
                    ctx,
                    text_response(
                        "Necesitas Administrar servidor y un envío pendiente de revisión.",
                    ),
                )
                .await?;
                return Ok(());
            }
            match action {
                "page" => {
                    let page = parts
                        .get(3)
                        .and_then(|v| v.parse::<i64>().ok())
                        .filter(|v| *v >= 0 && *v <= 10000)
                        .unwrap_or(0);
                    c.create_response(
                        ctx,
                        s::CreateInteractionResponse::UpdateMessage(
                            s::CreateInteractionResponseMessage::new()
                                .components(selection_menu(pool, id, page).await?),
                        ),
                    )
                    .await?;
                }
                "pick" => {
                    let s::ComponentInteractionDataKind::StringSelect { values } = &c.data.kind
                    else {
                        return Ok(());
                    };
                    let Some(game) = values.first().and_then(|v| v.parse::<i64>().ok()) else {
                        c.create_response(ctx, text_response("Partido inválido."))
                            .await?;
                        return Ok(());
                    };
                    let guild_id = i64::try_from(guild.get())?;
                    let row: Option<(i64,i64,i64,i64,i32,i64,i64)> = sqlx::query_as("SELECT s.tournament_id,s.source_channel_id,s.source_message_id,t.review_channel_id,g.fecha,g.home_id,g.away_id FROM match_submissions s JOIN tournaments t ON t.tournament_id=s.tournament_id JOIN tournament_matches g ON g.tournament_id=t.tournament_id WHERE s.submission_id=$1 AND s.author_id=$2 AND s.status='choosing' AND t.guild_id=$3 AND NOT t.finished AND g.match_id=$4 AND g.home_goals IS NULL AND (g.home_id=s.author_id OR g.away_id=s.author_id) AND t.review_channel_id IS NOT NULL").bind(id).bind(i64::try_from(c.user.id.get())?).bind(guild_id).bind(game).fetch_optional(pool).await?;
                    let Some((
                        tournament,
                        source_channel,
                        source_message,
                        review,
                        date,
                        home,
                        away,
                    )) = row
                    else {
                        c.create_response(
                            ctx,
                            text_response("Partido inválido, ya disputado o envío no disponible."),
                        )
                        .await?;
                        return Ok(());
                    };
                    let claimed = sqlx::query("UPDATE match_submissions SET status='sending',match_id=$1 WHERE submission_id=$2 AND status='choosing'").bind(game).bind(id).execute(pool).await?.rows_affected();
                    if claimed == 0 {
                        c.create_response(ctx, text_response("El envío ya está siendo procesado."))
                            .await?;
                        return Ok(());
                    }
                    c.create_response(
                        ctx,
                        s::CreateInteractionResponse::Defer(
                            s::CreateInteractionResponseMessage::new().ephemeral(true),
                        ),
                    )
                    .await?;
                    let send = async { let original = s::ChannelId::new(u64::try_from(source_channel)?).message(&ctx.http,s::MessageId::new(u64::try_from(source_message)?)).await?; let mut files = Vec::new(); for a in original.attachments.iter().filter(|a| a.content_type.as_deref().is_some_and(|t| t.starts_with("image/"))) { files.push(s::CreateAttachment::url(&ctx.http,&a.url).await?); } if files.is_empty() { return Err::<s::Message,Error>("No hay imágenes disponibles".into()); } let msg = s::ChannelId::new(u64::try_from(review)?).send_message(&ctx.http,s::CreateMessage::new().content(format!("Torneo `{tournament}` · fecha {date}\n<@{home}> (local) vs <@{away}> (visitante)\nEnviado por <@{}> · [mensaje original]({})",c.user.id,original.link())).files(files).components(buttons(id,true)).allowed_mentions(s::CreateAllowedMentions::new().empty_users().empty_roles())).await?; Ok(msg) }.await;
                    match send {
                        Ok(sent) => {
                            sqlx::query("UPDATE match_submissions SET status='pending',review_channel_id=$1,review_message_id=$2 WHERE submission_id=$3 AND status='sending'").bind(review).bind(i64::try_from(sent.id.get())?).bind(id).execute(pool).await?;
                            c.edit_response(
                                &ctx.http,
                                s::EditInteractionResponse::new()
                                    .content("Capturas enviadas para revisión."),
                            )
                            .await?;
                        }
                        Err(e) => {
                            sqlx::query("UPDATE match_submissions SET status='choosing',match_id=NULL WHERE submission_id=$1 AND status='sending'").bind(id).execute(pool).await?;
                            eprintln!("Error al reenviar capturas: {e}");
                            c.edit_response(&ctx.http,s::EditInteractionResponse::new().content("No pude reenviar las capturas. Comprueba los permisos y vuelve a intentarlo.")).await?;
                        }
                    }
                }
                "ignore" | "deny" => {
                    let new = if action == "ignore" {
                        "ignored"
                    } else {
                        "denied"
                    };
                    let old = if action == "ignore" {
                        "choosing"
                    } else {
                        "pending"
                    };
                    let changed = sqlx::query("UPDATE match_submissions SET status=$1 WHERE submission_id=$2 AND status=$3").bind(new).bind(id).bind(old).execute(pool).await?.rows_affected();
                    let note = if changed == 1 {
                        if action == "ignore" {
                            "Envío ignorado."
                        } else {
                            "Captura denegada."
                        }
                    } else {
                        "El envío ya fue procesado."
                    };
                    c.create_response(
                        ctx,
                        s::CreateInteractionResponse::UpdateMessage(
                            s::CreateInteractionResponseMessage::new()
                                .content(format!("{}\n\n**{}**", c.message.content, note))
                                .allowed_mentions(
                                    s::CreateAllowedMentions::new().empty_users().empty_roles(),
                                )
                                .components(vec![]),
                        ),
                    )
                    .await?;
                }
                "validate" => {
                    let game = game.ok_or("Partido ausente")?;
                    let (home,away): (String,String) = sqlx::query_as("SELECT h.username,a.username FROM tournament_matches g JOIN users h ON h.id=g.home_id JOIN users a ON a.id=g.away_id WHERE g.match_id=$1").bind(game).fetch_one(pool).await?;
                    c.create_response(ctx, score_modal(format!("sub:score:{id}"), &home, &away))
                        .await?;
                }
                _ => {}
            }
        }
        s::Interaction::Modal(m) if m.data.custom_id.starts_with("sub:") => {
            let parts: Vec<_> = m.data.custom_id.split(':').collect();
            let Some(id) = parts.get(2).and_then(|x| x.parse::<i64>().ok()) else {
                return Ok(());
            };
            let Some(guild) = m.guild_id else {
                return Ok(());
            };
            let guild_id = i64::try_from(guild.get())?;
            if parts[1] == "score" {
                if !admin(&m.member) {
                    m.create_response(ctx, text_response("Necesitas Administrar servidor."))
                        .await?;
                    return Ok(());
                }
                let goals = value(m, "home")
                    .and_then(|v| v.trim().parse::<i64>().ok())
                    .zip(value(m, "away").and_then(|v| v.trim().parse::<i64>().ok()))
                    .filter(|(a, b)| *a >= 0 && *b >= 0);
                let Some((hg, ag)) = goals else {
                    m.create_response(ctx,text_response("Los goles deben ser números enteros no negativos. Abre el formulario nuevamente.")).await?;
                    return Ok(());
                };
                let message_id = m
                    .message
                    .as_ref()
                    .map(|x| i64::try_from(x.id.get()))
                    .transpose()?;
                let game: Option<(i64,i32,i64,i64)> = sqlx::query_as("SELECT t.tournament_id,g.fecha,g.home_id,g.away_id FROM match_submissions s JOIN tournaments t ON t.tournament_id=s.tournament_id JOIN tournament_matches g ON g.match_id=s.match_id WHERE s.submission_id=$1 AND t.guild_id=$2 AND NOT t.finished AND s.status='pending' AND s.review_channel_id=$3 AND s.review_message_id=$4").bind(id).bind(guild_id).bind(i64::try_from(m.channel_id.get())?).bind(message_id).fetch_optional(pool).await?;
                let Some((t, date, home, away)) = game else {
                    m.create_response(
                        ctx,
                        text_response("El envío ya fue procesado o el torneo finalizó."),
                    )
                    .await?;
                    return Ok(());
                };
                let result = crate::database::repositories::tournaments::result_submission(
                    pool, guild_id, t, date, home, away, hg, ag, id,
                )
                .await?;
                let saved = matches!(
                    result,
                    crate::database::repositories::tournaments::MatchResult::Saved
                );
                let result_text = match result {
                    crate::database::repositories::tournaments::MatchResult::Saved => {
                        format!("Resultado validado: <@{home}> {hg} – {ag} <@{away}>.")
                    }
                    _ => {
                        "No se pudo registrar: el partido ya tiene resultado o no es único.".into()
                    }
                };
                m.create_response(ctx, text_response(&result_text)).await?;
                if saved {
                    if let Some(message) = &m.message {
                        if let Err(error) = m
                            .channel_id
                            .edit_message(
                                &ctx.http,
                                message.id,
                                s::EditMessage::new()
                                    .content(format!("{}\n\n**{}**", message.content, result_text))
                                    .allowed_mentions(
                                        s::CreateAllowedMentions::new().empty_users().empty_roles(),
                                    )
                                    .components(vec![]),
                            )
                            .await
                        {
                            eprintln!("No se pudo actualizar el mensaje de revisión: {error}");
                        }
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}
