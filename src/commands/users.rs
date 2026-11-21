use crate::database::repositories::users::get_user;
use crate::{Context, Error, serenity};

#[poise::command(slash_command)]
pub async fn user_info(ctx: Context<'_>, user: serenity::User) -> Result<(), Error> {
    match get_user(&ctx.data().pool, i64::try_from(user.id.get())?).await? {
        Some(u) => {
            ctx.say(format!("**{}** (ID {})\nVictorias: {} | Empates: {} | Derrotas: {} | WO: {}\nGoles a favor: {} | Goles en contra: {}\nTítulos oficiales: {} | Títulos amistosos: {}", u.username, u.id, u.pg, u.pe, u.pp, u.wo, u.gf, u.gc, u.titulos_oficiales, u.titulos_amistosos)).await?;
        }
        None => {
            ctx.say("Usuario no encontrado.").await?;
        }
    }
    Ok(())
}
