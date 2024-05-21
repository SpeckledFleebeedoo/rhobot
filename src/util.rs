use std::iter::once;
use poise::serenity_prelude as serenity;
use poise::reply::CreateReply;
use sqlx::{Pool, Sqlite};
use crate::{Context, Error, custom_errors::CustomError, Data, wiki_commands, mod_commands};
use regex::Regex;

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
    let modrole = match sqlx::query!(r#"SELECT modrole FROM servers WHERE server_id = ?1"#, server_id)
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

pub async fn escape_formatting(unformatted_string: String) -> String {
    // This is supposedly cheaper than using the String::replace function.
    unformatted_string
        .chars()
        .flat_map(|c| match c {
            '_' | '*' | '~' => Some('\\'),
            _ => None
        }
            .into_iter()
            .chain(once(c))
        )
        .flat_map(|c| once(c).chain( match c {
            '@' => Some('\u{200b}'),
            _ => None
        }))
        .collect::<String>()
}

pub async fn get_subscribed_mods(db: &Pool<Sqlite>, server_id: i64) -> Result<Vec<String>, Error> {
    let subscribed_mods = sqlx::query!(r#"SELECT mod_name FROM subscribed_mods WHERE server_id = ?1"#, server_id)
        .fetch_all(db)
        .await?
        .into_iter()
        .filter_map(|m| m.mod_name)
        .collect::<Vec<String>>();
    Ok(subscribed_mods)
}
pub async fn get_subscribed_authors(db: &Pool<Sqlite>, server_id: i64) -> Result<Vec<String>, Error> {
    let subscribed_authors = sqlx::query!(r#"SELECT author_name FROM subscribed_authors WHERE server_id = ?1"#, server_id)
        .fetch_all(db)
        .await?
        .into_iter()
        .filter_map(|m| m.author_name)
        .collect::<Vec<String>>();
    Ok(subscribed_authors)
}

/// Show stored information about this server
#[allow(clippy::cast_possible_wrap)]
#[poise::command(prefix_command, slash_command, guild_only, ephemeral, category="Settings")]
pub async fn get_server_info(
    ctx: Context<'_>
) -> Result<(), Error> {
    let Some(server) = ctx.guild_id() else {
        return Err(Box::new(CustomError::new("Could not get server ID")))
    };
    let server_id = server.get() as i64;
    
    let db = &ctx.data().database;
    let serverdata = sqlx::query!(r#"SELECT * FROM servers WHERE server_id = ?1"#, server_id)
        .fetch_optional(db)
        .await?;
    match serverdata {
        Some(data) => {
            let updates_channel = data.updates_channel.map_or_else(|| "Not set".to_owned(), |ch| format!("<#{ch}>"));
            let modrole = data.modrole.map_or_else(|| "Not set".to_owned(), |role| format!("<@&{role}>"));
            let show_changelog = data.show_changelog.map_or_else(|| "Not set (default to true)".to_owned(), |b| b.to_string());
            let response = format!("**Stored information for this server:**\nServer ID: {:?}\nUpdates channel: {}\nmodrole: {}\nShow changelogs: {}",
                data.server_id.unwrap_or(0), updates_channel, modrole, show_changelog);
            ctx.say(response).await?;
        },
        None => {
            ctx.say("No data stored about this server").await?;
        },
    }
    Ok(())
}

/// Show this help menu
#[poise::command(prefix_command, track_edits, slash_command)]
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

/// Remove all stored data for this server, resetting all settings.
#[allow(clippy::cast_possible_wrap)]
#[poise::command(prefix_command, slash_command, guild_only, category="Settings", check="is_mod")]
pub async fn reset_server_settings(
    ctx: Context<'_>
) -> Result<(), Error> {
    let Some(server) = ctx.guild_id() else {
        return Err(Box::new(CustomError::new("Could not get server ID")))
    };
    let server_id = server.get() as i64;
    let db = &ctx.data().database;
    sqlx::query!(r#"DELETE FROM servers WHERE server_id = ?1"#, server_id)
        .execute(db)
        .await?;
    ctx.say("Server data reset").await?;
    Ok(())
}

// /// Manually add entries to the database. Owner only.
// #[poise::command(prefix_command, slash_command, guild_only, owners_only, category="Management")]
// pub async fn migrate_serverdb_entry(
//     ctx: Context<'_>,
//     server_id: String,
//     updates_channel: Option<String>,
//     mod_role: Option<String>,
//     subscribed_mods: Option<String>,
// ) -> Result<(), Error> {
//     let db = &ctx.data().database;
//     let id = server_id.parse::<i64>().unwrap();
//     let ch = match updates_channel {
//         Some(c) => Some(c.parse::<i64>().unwrap()),
//         None => None,
//     };
//     let role = match mod_role {
//         Some(r) => Some(r.parse::<i64>().unwrap()),
//         None => None,
//     };

//     sqlx::query!(r#"INSERT INTO servers (server_id, updates_channel, modrole) VALUES (?1, ?2, ?3)"#, id, ch, role)
//         .execute(db)
//         .await?;
//     if subscribed_mods.is_some() {
//         let unwrapped_mods = subscribed_mods.unwrap();
//         let mods = unwrapped_mods.split(", ").collect::<Vec<&str>>();
//         for modname in mods {
//             sqlx::query!(r#"INSERT INTO subscribed_mods (server_id, mod_name) VALUES (?1, ?2)"#, server_id, modname)
//             .execute(db)
//             .await?;
//         };
//     };
//     ctx.say(format!("entry for server {server_id} added to database")).await?;
//     Ok(())
// }

#[allow(clippy::cast_possible_wrap)]
pub async fn on_guild_leave(id: serenity::GuildId, db: Pool<Sqlite>) -> Result<(), Error> {
    let server_id = id.get() as i64;
    sqlx::query!(r#"DELETE FROM servers WHERE server_id = ?1"#, server_id)
        .execute(&db)
        .await?;
    sqlx::query!(r#"DELETE FROM subscribed_mods WHERE server_id = ?1"#, server_id)
        .execute(&db)
        .await?;
    sqlx::query!(r#"DELETE FROM subscribed_authors WHERE server_id = ?1"#, server_id)
        .execute(&db)
        .await?;
    sqlx::query!(r#"DELETE FROM faq WHERE server_id = ?1"#, server_id)
        .execute(&db)
        .await?;
    println!("Left guild {server_id}");
    Ok(())
}
pub async fn send_custom_error_message(ctx: Context<'_>, msg: &str) -> Result<(), Error> {
    let embed = serenity::CreateEmbed::new()
        .title(format!("Error while executing command {}:", ctx.command().name))
        .description(msg)
        .color(serenity::Colour::RED);
    let builder = CreateReply::default()
        .embed(embed);
    ctx.send(builder).await?;
    Ok(())
}

#[allow(clippy::unnecessary_unwrap)]
pub async fn on_message(ctx: serenity::Context, msg: &serenity::Message, data: &Data) -> Result<(), Error> {
    if msg.author.bot {return Ok(())};
    println!("Handling message");
    let wiki_regex = Regex::new(r"\[\[(.*?)\]\]").unwrap();
    let neg_wiki_regex = Regex::new(r"\`[\S\s]*?\[\[(.*?)\]\][\S\s]*?\`").unwrap();
    let wiki_captures = wiki_regex.captures(&msg.content);
    if wiki_captures.is_some() {println!("Handling inline wiki command")};
    let neg_wiki_captures = neg_wiki_regex.captures(&msg.content);
    let wiki_search = if wiki_captures.is_none() || neg_wiki_captures.is_some() {
        None
    } else {
        Some(wiki_captures.unwrap()[1].to_owned())
    };
    
    let mod_regex = Regex::new(r">>(.*?)<<").unwrap();
    let neg_mod_regex = Regex::new(r"\`[\S\s]*?>>(.*?)<<[\S\s]*?\`").unwrap();
    let mod_captures = mod_regex.captures(&msg.content);
    if mod_captures.is_some() {println!("Handling inline mod command")};
    let neg_mod_captures = neg_mod_regex.captures(&msg.content);
    let mod_search = if mod_captures.is_none() || neg_mod_captures.is_some() {
        None
    } else {
        Some(mod_captures.unwrap()[1].to_owned())
    };

    if let Some(result_str) = wiki_search {
        let results = wiki_commands::opensearch_mediawiki(&result_str).await?;
        let Some(res) = results.first() else {
            println!("No results found");
            return Ok(())
        };
    
        let embed = wiki_commands::get_wiki_page(res).await?;
        let http = ctx.http.clone();
        let builder: serenity::CreateMessage = serenity::CreateMessage::new().embed(embed).reference_message(msg);
        msg.channel_id.send_message(http, builder).await?;
    };
    if let Some(result_str) = mod_search {
        let embed = mod_commands::mod_search(&result_str, true, data).await?;
        let http = ctx.http.clone();
        let builder: serenity::CreateMessage = serenity::CreateMessage::new().embed(embed).reference_message(msg);
        msg.channel_id.send_message(http, builder).await?;
    }
    Ok(())
}