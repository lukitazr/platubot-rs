use poise::serenity_prelude::{self as serenity};
use std::{
    collections::HashSet,
    sync::atomic::{AtomicU64, Ordering},
};

mod commands;
mod database;
mod events;
mod tournaments;

struct Data {
    contador: AtomicU64,
    pool: sqlx::Pool<sqlx::Postgres>,
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenvy::dotenv().ok();

    let token = std::env::var("DISCORD_TOKEN")?;

    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let guild_id = serenity::GuildId::new(1090431941184082010);

    let mut owners = HashSet::new();
    owners.insert(serenity::UserId::new(1052388988368990279));

    let pool: sqlx::Pool<sqlx::Postgres> = database::connect().await?;
    println!("Conectado a PostgreSQL correctamente");
    sqlx::migrate!().run(&pool).await?;
    sqlx::query(
        "UPDATE match_submissions SET status='choosing', match_id=NULL WHERE status='sending'",
    )
    .execute(&pool)
    .await?;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands::all(),
            event_handler: |ctx, event| {
                Box::pin(async move {
                    if let serenity::FullEvent::Message { new_message } = event {
                        let data = ctx.user_data().await;

                        events::message::message_event(new_message, &data.pool).await;
                        if let Err(e) = events::submissions::message(
                            ctx.serenity_context,
                            new_message,
                            &data.pool,
                        )
                        .await
                        {
                            eprintln!("Error al procesar capturas: {e}");
                        }
                    }

                    if let serenity::FullEvent::InteractionCreate { interaction } = event {
                        let data = ctx.user_data().await;
                        if let Err(e) = events::results::interaction(
                            ctx.serenity_context,
                            interaction,
                            &data.pool,
                        )
                        .await
                        {
                            eprintln!("Error en formulario de resultado: {e}");
                        }
                        if let Err(e) = events::submissions::interaction(
                            ctx.serenity_context,
                            interaction,
                            &data.pool,
                        )
                        .await
                        {
                            eprintln!("Error en interaccion de capturas: {e}");
                        }
                    }
                    Ok(())
                })
            },
            owners,
            ..Default::default()
        })
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                println!("Conectado como {}", ready.user.name);

                poise::builtins::register_in_guild(ctx, &framework.options().commands, guild_id)
                    .await?;

                Ok(Data {
                    contador: AtomicU64::new(0),
                    pool,
                })
            })
        })
        .build();

    let mut client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await?;

    client.start().await?;

    Ok(())
}
