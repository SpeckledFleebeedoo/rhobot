use poise::serenity_prelude as serenity;

use crate::{
    custom_errors::CustomError,
    database,
    Context,
    Error
};

#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub async fn is_mod(ctx: Context<'_>) -> Result<bool, Error> {

    let Some(channel) = &ctx.guild_channel().await
        else {return Ok(false)} ;
    let Some(member) = &ctx.author_member().await
        else {return Ok(false)};
    let Some(guild) = ctx.partial_guild().await
        else {return Ok(false)};
    let user_permissions = guild.user_permissions_in(channel, member);
    if user_permissions.contains(serenity::Permissions::ADMINISTRATOR) {
        return Ok(true);
    };
    let db = &ctx.data().database;
    let Some(server) = ctx.guild_id() else {
        return Err(Box::new(CustomError::new("Could not get server ID")))
    };
    let server_id = server.get() as i64;
    let Some(modrole) = database::get_modrole(db, server_id).await? else {
        return Ok(false);
    };
    let has_role = ctx.author().has_role(ctx.http(), server, serenity::RoleId::from(modrole as u64)).await?;
    Ok(has_role)
}