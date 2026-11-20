use crate::database::repositories::users::get_user;
use crate::serenity;
use crate::Context;
use crate::Error;

#[poise::command(slash_command)]
pub async fn user_info(
    ctx: Context<'_>,
    user: serenity::User,
) -> Result<(), Error> {
    let user_id: i64 = i64::try_from(user.id.get())?;
    let res = get_user(
        &ctx.data().pool, user_id
    ).await;

    match res {
        Ok(Some(user_data)) => {
            ctx.say(
                format!(
                    "**Información del usuario**\n\
                    ID: {}\n\
                    Username: {}\n\
                    ",
                    user_data.id,
                    user_data.username,
                    // user_data.level,
                    // user_data.experience
                )
            )
            .await?;
        },
        Ok(None) => {
            ctx.say(
                "Usuario no encontrado!"
            )
            .await?;
        },
        Err(e) => {
            ctx.say(
                "Error en la base de datos al intentar buscar al usuario."
            )
            .await?;
            println!("{:#?}", e);
        },
    }

    Ok(())
}