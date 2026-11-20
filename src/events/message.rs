use poise::serenity_prelude::Message;
use sqlx::PgPool;

use crate::database::repositories::users::create_users;
use crate::database::repositories::guild_users::create_guild_user;

pub async fn message_event(
    new_message: &Message,
    pool: &PgPool,
) {
    if new_message.author.bot {
        return;
    }

    // Ignorar mensajes privados
    let Some(gid) = new_message.guild_id else {
        return;
    };

    let Ok(id) = i64::try_from(new_message.author.id.get()) else {
        eprintln!("ID del usuario fuera del rango de i64");
        return;
    };

    let Ok(guild_id) = i64::try_from(gid.get()) else {
        eprintln!("ID del servidor fuera del rango de i64");
        return;
    };

    let username = new_message
        .author
        .global_name
        .as_deref()
        .unwrap_or(&new_message.author.name);

    // Registro global
    if let Err(e) = create_users(pool, id, username).await {
        eprintln!("Error al registrar usuario: {e}");
        return;
    }

    // Registro por servidor
    if let Err(e) = create_guild_user(pool, guild_id, id).await {
        eprintln!("Error al registrar usuario en servidor: {e}");
        return;
    }

    println!(
        "Registro procesado: {} en servidor {}",
        username, guild_id
    );
}