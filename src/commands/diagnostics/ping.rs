use crate::Context;
use crate::Error;
use crate::Ordering;
use bot_command_macros::bot_command;

/// Comprueba la latencia del bot y muestra el contador de respuestas.
#[bot_command(slash_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    ctx.data().contador.fetch_add(1, Ordering::Relaxed);
    ctx.say(format!(
        "Pong! {}",
        ctx.data().contador.load(Ordering::Relaxed)
    ))
    .await?;

    Ok(())
}
