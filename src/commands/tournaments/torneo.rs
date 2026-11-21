use super::{crear, finalizar, fixture, inscribir, programar, resultado, sortear, tabla, ver, wo};
use crate::{Context, Error};
use bot_command_macros::bot_command;

/// Agrupa las operaciones para administrar y consultar torneos.
#[bot_command(
    slash_command,
    guild_only,
    subcommands(
        "crear",
        "inscribir",
        "programar",
        "sortear",
        "resultado",
        "wo",
        "ver",
        "finalizar",
        "fixture",
        "tabla"
    )
)]
pub async fn torneo(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}
