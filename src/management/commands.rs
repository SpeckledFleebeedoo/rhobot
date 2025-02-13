use poise::serenity_prelude as serenity;
use poise::CreateReply;

use crate::{
    Context,
    Error,
    management::{get_server_id, checks::is_mod},
    database,
};

/// Remove all stored data for this server, resetting all settings.
#[poise::command(prefix_command, slash_command, guild_only, category="Settings", check="is_mod")]
pub async fn reset_server_settings(
    ctx: Context<'_>
) -> Result<(), Error> {
    let server_id = get_server_id(ctx)?;
    let db = &ctx.data().database;
    database::clear_server_data(server_id, db).await?;
    ctx.say("Server data reset").await?;
    Ok(())
}

/// Print bot info
#[poise::command(prefix_command, slash_command, install_context = "Guild|User", interaction_context = "Guild|BotDm|PrivateChannel")]
pub async fn info(
    ctx: Context<'_>
) -> Result<(), Error> {
    let embed = serenity::CreateEmbed::new()
        .title("ρBot")
        .field("Creator", "SpeckledFleebeedoo (<@247640901805932544>)", false)
        .field("Source", "[GitHub](https://www.github.com/SpeckledFleebeedoo/rhobot)", true)
        .field("Invite link", "[Invite](https://discord.com/api/oauth2/authorize?client_id=872540831599456296&permissions=274877925376&scope=bot%20applications.commands)", true);
    let builder = CreateReply::default().embed(embed);
    ctx.send(builder).await?;
    Ok(())
}

/// Show this help menu
#[poise::command(prefix_command, track_edits, slash_command, install_context = "Guild|User", interaction_context = "Guild|BotDm|PrivateChannel")]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "poise::builtins::autocomplete_command"]
    command: Option<String>,
) -> Result<(), Error> {
    poise::builtins::help(
        ctx,
        command.as_deref(),
        poise::builtins::HelpConfiguration::default(),
    )
    .await?;
    Ok(())
}

/// Show stored information about this server
#[poise::command(prefix_command, slash_command, guild_only, ephemeral, category="Settings")]
pub async fn get_server_info(
    ctx: Context<'_>
) -> Result<(), Error> {
    let server_id = get_server_id(ctx)?;
    
    let db = &ctx.data().database;
    let serverdata = database::get_server_info(db, server_id).await?;
    match serverdata {
        Some(data) => {
            let updates_channel = data.updates_channel.map_or_else(|| "Not set".to_owned(), |ch| format!("<#{ch}>"));
            let modrole = data.modrole.map_or_else(|| "Not set".to_owned(), |role| format!("<@&{role}>"));
            let show_changelog = data.show_changelog.map_or_else(|| "Not set (default to true)".to_owned(), |b| b.to_string());
            let response = format!("**Stored information for this server:**\nServer ID: {:?}\nUpdates channel: {}\nmodrole: {}\nShow changelogs: {}",
                data.server_id, updates_channel, modrole, show_changelog);
            ctx.say(response).await?;
        },
        None => {
            ctx.say("No data stored about this server").await?;
        },
    }
    Ok(())
}