use poise::serenity_prelude as serenity;

use crate::{
    Context,
    custom_errors::CustomError,
    Error,
};

#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub async fn is_mod(ctx: Context<'_>) -> Result<bool, Error> {
    let user_permissions = match ctx.author_member().await{
        Some(p) => p.permissions(ctx.cache())?,
        None => serenity::Permissions::empty(), // Assume user has no permissions
    };
    if user_permissions.contains(serenity::Permissions::ADMINISTRATOR) {
        return Ok(true);
    };
    let db = &ctx.data().database;
    let Some(server) = ctx.guild_id() else {
        return Err(Box::new(CustomError::new("Could not get server ID")))
    };
    let server_id = server.get() as i64;
    let modrole = match sqlx::query!(r#"SELECT modrole FROM servers WHERE server_id = $1"#, server_id)
        .fetch_one(db)
        .await {
            Ok(role) => {match role.modrole {
                Some(role) => role,
                None => {
                    return Ok(false)
                },
            }},
            Err(_) => {
                return Ok(false)
            },
        };
    let has_role = ctx.author().has_role(ctx.http(), server, serenity::RoleId::from(modrole as u64)).await?;
    Ok(has_role)
}