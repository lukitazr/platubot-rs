use crate::{Context, Error};
use bot_command_macros::bot_command;

/// Comprueba la conexión con la base de datos. Solo para propietarios del bot.
#[bot_command(slash_command, owners_only)]
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
