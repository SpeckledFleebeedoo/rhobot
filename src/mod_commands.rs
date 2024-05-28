use poise::serenity_prelude::{AutocompleteChoice, CreateEmbed, Colour};
use poise::CreateReply;
use rust_fuzzy_search::fuzzy_search_best_n;
use log::error;

use crate::{Context, Error, custom_errors::CustomError, Data, SEPARATOR,
    util::{escape_formatting, get_subscribed_authors, get_subscribed_mods, is_mod, get_server_id},
    mods::{self, ModCacheEntry, SubCacheEntry, SubscriptionType}
};

enum AutocompleteType{
    Mod,
    Author,
}

/// Set the channel to send mod update messages to. Bot will not work without one.
#[allow(clippy::cast_possible_wrap)]
#[poise::command(prefix_command, slash_command, guild_only, check="is_mod", category="Settings")]
pub async fn set_updates_channel(
    ctx: Context<'_>,
    channel: poise::serenity_prelude::GuildChannel,
) -> Result<(), Error> {
    let channel_id = channel.id.get() as i64;
    let server_id = channel.guild_id.get() as i64;
    let db = &ctx.data().database;

    if (sqlx::query!(r#"SELECT * FROM servers WHERE server_id = $1"#, server_id)
        .fetch_optional(db)
        .await?).is_some() {
        // Update server data if it does exist
        sqlx::query!(r#"UPDATE servers SET updates_channel = $1 WHERE server_id = $2"#,
        channel_id, server_id)
            .execute(db)
            .await?;
    } else {
        // Add server and set setting if it does not exist
        sqlx::query!(r#"INSERT INTO servers (server_id, updates_channel) VALUES ($1, $2)"#,
        server_id, channel_id)
            .execute(db)
            .await?;
    }

    let response = format!("Mod updates channel was set to {channel}");
    ctx.say(response).await?;
    Ok(())
}

/// Set which role is allowed to edit bot settings. Admins can always edit settings.
#[allow(clippy::cast_possible_wrap)]
#[poise::command(prefix_command, slash_command, guild_only, check="is_mod", category="Settings")]
pub async fn set_modrole(
    ctx: Context<'_>,
    role: poise::serenity_prelude::Role,
) -> Result<(), Error> {
    let role_id = role.id.get() as i64;
    let server_id = role.guild_id.get() as i64;
    let db = &ctx.data().database;
    
    if (sqlx::query!(r#"SELECT * FROM servers WHERE server_id = $1"#, server_id)
        .fetch_optional(db)
        .await?).is_some() {
        // Update server data if it does exist
        sqlx::query!(r#"UPDATE servers SET modrole = $1 WHERE server_id = $2"#,
        role_id, server_id)
            .execute(db)
            .await?;
    } else {
        // Add server and set setting if it does not exist
        sqlx::query!(r#"INSERT INTO servers (server_id, modrole) VALUES ($1, $2)"#,
        server_id, role_id)
            .execute(db)
            .await?;
    }

    let response = format!("Modrole was set to {role}");
    ctx.say(response).await?;
    Ok(())
}

/// Turn showing changelogs in update feed on or off
#[poise::command(prefix_command, slash_command, guild_only, check="is_mod", category="Settings")]
pub async fn show_changelogs(
    ctx: Context<'_>,
    show_changelogs: bool,
) -> Result<(), Error> {
    let server_id = get_server_id(ctx)?;
    let db = &ctx.data().database;
    match sqlx::query!(r#"SELECT server_id FROM servers WHERE server_id = $1"#, server_id)
            .fetch_optional(db)
            .await? {
        Some(_) => {
            // Update server data if it does exist
            sqlx::query!(r#"UPDATE servers SET show_changelog = $1 WHERE server_id = $2"#, 
            show_changelogs, server_id)
            .execute(db)
            .await?;
        },
        None => {
            // Add server and set setting if it does not exist
            sqlx::query!(r#"INSERT INTO servers (server_id, show_changelog) VALUES ($1, $2)"#,
            server_id, show_changelogs)
            .execute(db)
            .await?;
        },
    };
    if show_changelogs { ctx.say("Now showing changelogs in mod updates feed.").await?
    } else { ctx.say("No longer showing changelogs in mod updates feed.").await? };
    Ok(())
}

/// Unsubscribe from a mod or author.
#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, guild_only, check="is_mod", subcommands("unsubscribe_author", "unsubscribe_mod"), subcommand_required, category="Subscriptions")]
pub async fn unsubscribe(
    _: Context<'_>
) -> Result<(), Error> {
    Ok(())
}

/// Subscribe to a mod or author. Only subscriptions are shown in the update feed.
#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, guild_only, check="is_mod", subcommands("subscribe_author", "subscribe_mod"), subcommand_required, category="Subscriptions")]
pub async fn subscribe(
    _: Context<'_>
) -> Result<(), Error> {
    Ok(())
}

/// Subscribe to a mod
#[allow(clippy::unused_async, clippy::cast_possible_wrap)]
#[poise::command(prefix_command, slash_command, guild_only, check="is_mod", rename="mod")]
pub async fn subscribe_mod(
    ctx: Context<'_>,
    #[description = "Name of the mod to subscribe to"]
    #[autocomplete = "autocomplete_modname"]
    modname: String,
) -> Result<(), Error> {
    let Some(server) = ctx.guild_id() else {
        return Err(Box::new(CustomError::new("Could not get server ID")))
    };
    let server_id = server.get() as i64;
    let db = &ctx.data().database;

    sqlx::query!(r#"INSERT OR REPLACE INTO subscribed_mods (server_id, mod_name) VALUES ($1, $2)"#, server_id, modname)
        .execute(db)
        .await?;
    ctx.say(format!("Mod {modname} added to subscriptions")).await?;

    let cache = &ctx.data().mod_subscription_cache;
    match cache.write() {
        Ok(mut c) => c.push(
            SubCacheEntry{
                server_id,
                subscription: SubscriptionType::Modname(modname),
            }
        ),
        Err(e) => {
            return Err(Box::new(CustomError::new(&format!("Error acquiring cache: {e}"))));
        }
    }
    Ok(())
}


/// Unsubscribe from a mod
#[allow(clippy::unused_async, clippy::cast_possible_wrap)]
#[poise::command(prefix_command, slash_command, guild_only, check="is_mod", rename="mod")]
pub async fn unsubscribe_mod(
    ctx: Context<'_>,
    #[description = "Name of the mod to unsubscribe from"]
    #[autocomplete = "autocomplete_subscribed_modname"]
    modname: String,
) -> Result<(), Error> {
    let Some(server) = ctx.guild_id() else {
        return Err(Box::new(CustomError::new("Could not get server ID")))
    };
    let server_id = server.get() as i64;
    let db = &ctx.data().database;
    sqlx::query!(r#"DELETE FROM subscribed_mods WHERE server_id = $1 AND mod_name = $2"#, server_id, modname)
        .execute(db)
        .await?;
    let response = format!("Mod {modname} removed from subscriptions");
    ctx.say(response).await?;
    Ok(())
}

#[allow(clippy::unused_async)]
async fn autocomplete_subscribed_modname(
    ctx: Context<'_>,
    partial: &str,
) -> Vec<String> {
    autocomplete_unsubscribe(ctx, partial, &AutocompleteType::Mod)
}

/// Subscribe to a mod author
#[allow(clippy::unused_async, clippy::cast_possible_wrap)]
#[poise::command(prefix_command, slash_command, guild_only, check="is_mod", rename="author")]
pub async fn subscribe_author(
    ctx: Context<'_>,
    #[description = "Name of the mod author to subscribe to"]
    #[autocomplete = "autocomplete_author"]
    author: String,
) -> Result<(), Error> {
    let Some(server) = ctx.guild_id() else {
        return Err(Box::new(CustomError::new("Could not get server ID")))
    };
    let server_id = server.get() as i64;
    let db = &ctx.data().database;

    sqlx::query!(r#"INSERT OR REPLACE INTO subscribed_authors (server_id, author_name) VALUES ($1, $2)"#, server_id, author)
        .execute(db)
        .await?;
    let response = format!("Author {author} added to subscriptions");
    ctx.say(response).await?;

    let cache = &ctx.data().mod_subscription_cache;
    match cache.write() {
        Ok(mut c) => c.push(
            SubCacheEntry{
                server_id,
                subscription: SubscriptionType::Author(author),
            }
        ),
        Err(e) => {
            return Err(Box::new(CustomError::new(&format!("Error acquiring cache: {e}"))));
        }
    }
    Ok(())
}

#[allow(clippy::unused_async)]
async fn autocomplete_author(
    ctx: Context<'_>,
    partial: &str,
) -> Vec<String> {
    let cache = &ctx.data().mod_author_cache;
    let author_cache = match cache.read(){
        Ok(c) => c,
        Err(e) => {
            error!{"Error acquiring cache: {e}"}
            return vec![]
        },
    }.clone();
    author_cache.into_iter()
        .filter(|entry| entry.starts_with(partial))
        .collect::<Vec<String>>()
}

/// Unsubscribe from a mod author
#[allow(clippy::unused_async, clippy::cast_possible_wrap)]
#[poise::command(prefix_command, slash_command, guild_only, check="is_mod", rename="author")]
pub async fn unsubscribe_author(
    ctx: Context<'_>,
    #[description = "Name of the mod author to unsubscribe from"]
    #[autocomplete = "autocomplete_subscribed_author"]
    author: String,
) -> Result<(), Error> {
    let Some(server) = ctx.guild_id() else {
        return Err(Box::new(CustomError::new("Could not get server ID")))
    };
    let server_id = server.get() as i64;
    let db = &ctx.data().database;
    sqlx::query!(r#"DELETE FROM subscribed_authors WHERE server_id = $1 AND author_name = $2"#, server_id, author)
        .execute(db)
        .await?;
    let response = format!("Author {author} removed from subscriptions");
    ctx.say(response).await?;
    Ok(())
}

#[allow(clippy::unused_async)]
async fn autocomplete_subscribed_author(
    ctx: Context<'_>,
    partial: &str,
) -> Vec<String> {
    autocomplete_unsubscribe(ctx, partial, &AutocompleteType::Author)
}
#[allow(clippy::cast_possible_wrap)]
fn autocomplete_unsubscribe(
    ctx: Context<'_>,
    partial: &str,
    data_type: &AutocompleteType,
) -> Vec<String> {
    let cache = &ctx.data().mod_subscription_cache;
    let Some(server) = ctx.guild_id() else {
        error!("Could not get server ID while autocompleting faq name"); 
        return vec![]
    };
    let server_id = server.get() as i64;
    let subscription_cache = match cache.read(){
        Ok(c) => c,
        Err(e) => {
            error!{"Error acquiring cache: {e}"}
            return vec![]
        },
    };
    match data_type {
        AutocompleteType::Mod => {
            subscription_cache.clone()
                .into_iter()
                .filter(|entry| entry.server_id == server_id)
                .filter_map(|entry| match entry.subscription {
                    SubscriptionType::Author(_) => None,
                    SubscriptionType::Modname(name) => Some(name),
                })
                .filter(|entry| entry.starts_with(partial))
                .collect::<Vec<String>>()
        },
        AutocompleteType::Author => {
            subscription_cache.clone()
                .into_iter()
                .filter(|entry| entry.server_id == server_id)
                .filter_map(|entry| match entry.subscription {
                    SubscriptionType::Author(name) => Some(name),
                    SubscriptionType::Modname(_) => None,
                })
                .filter(|entry| entry.starts_with(partial))
                .collect::<Vec<String>>()
        },
    }
}

/// List which mods and authors the server is currently subscribed to.
#[allow(clippy::unused_async, clippy::cast_possible_wrap)]
#[poise::command(prefix_command, slash_command, guild_only, category="Subscriptions")]
pub async fn show_subscriptions(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let Some(server) = ctx.guild_id() else {
        return Err(Box::new(CustomError::new("Could not get server ID")))
    };
    let server_id = server.get() as i64;
    let db = &ctx.data().database;

    let subscribed_mods = get_subscribed_mods(db, server_id)
        .await?
        .join("\n");

    let subscribed_authors = get_subscribed_authors(db, server_id)
        .await?
        .join("\n");

    let response = format!("**Subscribed mods:**\n{subscribed_mods}\n**Subscribed authors:**\n{subscribed_authors}");
    ctx.say(response).await?;
    Ok(())
}

/// Find a mod on the mod portal.
#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, track_edits, rename="mod", aliases("find-mod", "find_mod"))]
pub async fn find_mod(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_modname"]
    #[description = "Name of the mod to search for"]
    #[rest]
    modname: String,
) -> Result<(), Error> {
    let command = modname.split(SEPARATOR).next().unwrap_or(&modname);
    let embed = match ctx {
        poise::Context::Application(_) => mod_search(command, false, ctx.data()).await?,
        poise::Context::Prefix(_) => mod_search(command, true, ctx.data()).await?,
    };
    let builder = CreateReply::default().embed(embed);
    ctx.send(builder).await?;
    Ok(())
}

pub async fn mod_search(modname: &str, imprecise_search: bool, data: &Data) -> Result<CreateEmbed, Error> {
    let search_result = if imprecise_search {
        let cache = data.mod_cache.clone();
        let modcache = match cache.read() {
            Ok(c) => c,
            Err(e) => {
                return Err(Box::new(CustomError::new(&format!("Error acquiring cache: {e}"))));
            },
        }.clone();
        let title_list = modcache.clone()
            .into_iter()
            .map(|f| f.title)
            .collect::<Vec<String>>();
        let title_list_unowned = title_list.iter()
            .map(std::convert::AsRef::as_ref)
            .collect::<Vec<&str>>();
        let query = modname.split('|').collect::<Vec<&str>>()[0];
        let title = fuzzy_search_best_n(query, &title_list_unowned, 1)[0].0.to_owned();
        let found_name = modcache.into_iter()
            .filter(|f| f.title == title)
            .collect::<Vec<ModCacheEntry>>();
        found_name[0].clone().name
    } else {
        modname.to_owned()
    };

    let db = &data.database;

    let Ok(mod_data) = sqlx::query!(r#"SELECT * FROM mods WHERE name = $1"#, search_result)
        .fetch_one(db)
        .await else {
                return Err(Box::new(CustomError::new( "Failed to find mod in database")));
        };

    let name = mod_data.name;
    let mut title = escape_formatting(&mod_data.title.unwrap_or_else(|| name.clone())).await;
    title.truncate(256);
    let thumbnail = mods::get_mod_thumbnail(&name).await.unwrap_or_else(|_| "/assets/.thumb.png".to_owned());
    let url = format!("https://mods.factorio.com/mod/{name}")
        .replace(' ', "%20");
    let mut summary = escape_formatting(&mod_data.summary.unwrap_or(String::new())).await;
    summary.truncate(4096);
    let owner = escape_formatting(&mod_data.owner).await;
    let downloads = mod_data.downloads_count.to_string();
    let color = Colour::from_rgb(0x2E, 0xCC, 0x71);

    let embed = CreateEmbed::new()
        .title(&title)
        .url(url)
        .description(&summary)
        .color(color)
        .field("Author", owner, true)
        .field("Downloads", downloads, true)
        .thumbnail(&thumbnail);
    Ok(embed)
}

#[allow(clippy::unused_async)]
async fn autocomplete_modname<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<AutocompleteChoice> {
    let mut listed_names: Vec<String> = Vec::new();

    let cache = ctx.data().mod_cache.clone();
    let modcache = match cache.read(){
        Ok(c) => c,
        Err(e) => {
            error!{"Error acquiring cache: {e}"}
            return vec![]
        },
    }.clone();
    let mut list = modcache.clone().into_iter()
        .filter(move |f| 
            f.title.to_lowercase().starts_with(&partial.to_lowercase()) 
            || f.author.to_lowercase().starts_with(&partial.to_lowercase())
        )
        .map(|f| {
            listed_names.push(f.name.clone());
            AutocompleteChoice::new(
                f.title + " by " + &f.author,
                f.name,
            )
        })
        .collect::<Vec<AutocompleteChoice>>();
    if list.len() >= 25 {
        return list;
    };

    let mut title_contains = modcache.iter()
        .filter(|f| 
            !(listed_names.contains(&f.name))  // Exclude previously found names
            && f.title.to_lowercase().contains(&partial.to_lowercase()))
        .map(|f| {
            AutocompleteChoice::new(
                f.title.clone() + " by " + &f.author,
                f.name.clone(),
            )
        })
        .collect::<Vec<AutocompleteChoice>>();
    list.append(&mut title_contains);
    if list.len() >= 25 {
        return list;
    };

    let mut name_contains = modcache.iter()
    .filter(|f| 
        !(listed_names.contains(&f.name))  // Exclude previously found names
        && f.name.to_lowercase().contains(&partial.to_lowercase()))
    .map(|f| {
        AutocompleteChoice::new(
            f.title.clone() + " by " + &f.author,
            f.name.clone(),
        )
    })
    .collect::<Vec<AutocompleteChoice>>();
    list.append(&mut name_contains);

    list
}
