use poise::serenity_prelude as serenity;

use crate::{Context, Error, database, management::ManagementError};

#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub async fn is_mod(ctx: Context<'_>) -> Result<bool, Error> {
    let Some(channel) = &ctx.guild_channel().await else {
        return Ok(false);
    };
    let Some(member) = &ctx.author_member().await else {
        return Ok(false);
    };
    let Some(guild) = ctx.partial_guild().await else {
        return Ok(false);
    };
    let user_permissions = guild.user_permissions_in(channel, member);
    if user_permissions.contains(serenity::Permissions::ADMINISTRATOR) {
        return Ok(true);
    }
    let db = &ctx.data().database;
    let server = ctx
        .guild_id()
        .ok_or_else(|| ManagementError::ServerNotFound)?;
    let server_id = server.get() as i64;
    let Some(modrole) = database::get_modrole(db, server_id).await? else {
        return Ok(false);
    };
    let has_role = ctx
        .author()
        .has_role(ctx.http(), server, serenity::RoleId::from(modrole as u64))
        .await?;
    Ok(has_role)
}

pub async fn is_owner(ctx: Context<'_>, user: serenity::User) -> Result<bool, ManagementError> {
    let Some(owner) = ctx.http().get_current_application_info().await?.owner else {
        return Err(ManagementError::OwnerVerificationFailed)?;
    };
    Ok(user == owner)
}
