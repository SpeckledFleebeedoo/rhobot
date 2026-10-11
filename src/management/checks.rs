use poise::serenity_prelude as serenity;
use sqlx::{Pool, Sqlite};

use crate::{Context, Data, Error, database, management::ManagementError};

#[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]
pub async fn is_mod(ctx: poise::Context<'_, Data, Error>) -> Result<bool, Error> {
    let Some(serenity::Channel::Guild(channel)) = &ctx.channel().await else {
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
    let server_id = guild.id.get() as i64;
    let Some(modrole) = database::get_modrole(db, server_id).await? else {
        return Ok(false);
    };
    let has_role = member
        .user
        .has_role(ctx.http(), guild.id, serenity::RoleId::from(modrole as u64))
        .await?;
    Ok(has_role)
}

pub async fn is_mod_interaction(
    interaction: &serenity::ComponentInteraction,
    db: &Pool<Sqlite>,
) -> bool {
    let Some(ref member) = interaction.member else {
        return false;
    };
    let Some(permissions) = member.permissions else {
        return false;
    };
    if permissions.contains(serenity::Permissions::ADMINISTRATOR) {
        return true;
    }
    let Some(guild) = interaction.guild_id else {
        return false;
    };
    let server_id = guild.get().cast_signed();
    let Ok(Some(modrole)) = database::get_modrole(db, server_id).await else {
        return false;
    };
    if member
        .roles
        .contains(&serenity::RoleId::from(modrole.cast_unsigned()))
    {
        return true;
    }
    false
}

pub async fn is_owner(ctx: Context<'_>, user: serenity::User) -> Result<bool, ManagementError> {
    let Some(owner) = ctx.http().get_current_application_info().await?.owner else {
        return Err(ManagementError::OwnerVerificationFailed)?;
    };
    Ok(user == owner)
}
