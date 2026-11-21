use crate::Context;
use crate::Error;
use crate::Ordering;

#[poise::command(slash_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.data().contador.fetch_add(1, Ordering::Relaxed);
    ctx.say(format!(
        "Pong! {}",
        ctx.data().contador.load(Ordering::Relaxed)
    ))
    .await?;

    Ok(())
}

#[poise::command(slash_command, owners_only)]
pub async fn db_ping(ctx: Context<'_>) -> Result<(), Error> {
    let result = sqlx::query("SELECT 1").execute(&ctx.data().pool).await;

    match result {
        Ok(res) => {
            ctx.say(format!("Base de datos lista!")).await?;
            println!("{:#?}", res)
        }
        Err(e) => {
            ctx.say(format!("Error en la base de datos!")).await?;
            println!("{:#?}", e);
        }
    }

    Ok(())
}
