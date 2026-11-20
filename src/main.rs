use poise::serenity_prelude::{self as serenity};
use std::{collections::HashSet, sync::atomic::{AtomicU64, Ordering}};

mod commands;
mod events;
mod database;

struct Data {
    contador: AtomicU64,
    pool: sqlx::Pool<sqlx::Postgres>
}

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenvy::dotenv().ok();

    let token = std::env::var("DISCORD_TOKEN")?;

    let intents = serenity::GatewayIntents::non_privileged() |
    serenity::GatewayIntents::MESSAGE_CONTENT;

    let guild_id = serenity::GuildId::new(1090431941184082010);

    let mut owners = HashSet::new();
    owners.insert(serenity::UserId::new(1052388988368990279));

    let pool: sqlx::Pool<sqlx::Postgres> = database::connect().await?;
    println!("Conectado a PostgreSQL correctamente");
    sqlx::migrate!().run(&pool).await?;

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::ping::ping(),
                commands::ping::db_ping(),
                commands::stress_test::stress_test(),
                commands::users::user_info(),
                commands::tournaments::torneo()
            ], 
            event_handler: |ctx, event| {
                 Box::pin(async move {
                    if let serenity::FullEvent::Message { new_message } = event {
                        let data = ctx.user_data().await;

                        events::message::message_event(
                            new_message,
                            &data.pool,
                        )
                        .await;
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

                poise::builtins::register_in_guild(
                    ctx,
                    &framework.options().commands,
                    guild_id,
                )
                .await?;

                Ok(Data {
                    contador: AtomicU64::new(0),
                    pool
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
