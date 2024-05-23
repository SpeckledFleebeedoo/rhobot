use sqlx::{Pool, Sqlite};
use std::sync::{Arc, RwLock};
use poise::serenity_prelude::{CreateEmbed, Colour};
use poise::CreateReply;
use log::error;

use crate::{Context, Error, custom_errors::CustomError};

#[derive(Debug, Clone)]
pub struct FaqCacheEntry {
    server_id: i64,
    name: String,
}

pub async fn update_faq_cache(
    cache: Arc<RwLock<Vec<FaqCacheEntry>>>,
    db: Pool<Sqlite>
) -> Result<(), Error> {
    let records = sqlx::query!(r#"SELECT * FROM faq"#)
        .fetch_all(&db)
        .await?
        .iter()
        .map(|r| {
            FaqCacheEntry{
                server_id: r.server_id.unwrap_or_default(), // Return 0 in case of issues
                name: r.title.clone().unwrap_or_default(),  // Return empty string in case of issues
            }
        })
        .collect::<Vec<FaqCacheEntry>>();

    match cache.write() {
        Ok(mut c) => {*c = records},
        Err(e) => {
            return Err(Box::new(CustomError::new(&format!("Error acquiring cache: {e}"))));
        },
    };
    Ok(())
}

/// Frequently Asked Questions
#[allow(clippy::unused_async, clippy::cast_possible_wrap)]
#[poise::command(slash_command, guild_only, rename = "faq")]
pub async fn faq_slash(
    ctx: Context<'_>,
    #[description = "Name of the faq entry"]
    #[autocomplete = "autocomplete_faq"]
    name: String,
) -> Result<(), Error> {
    faq_core(ctx, name).await?;
    Ok(())
}

/// Frequently Asked Questions
#[allow(clippy::unused_async, clippy::cast_possible_wrap)]
#[poise::command(prefix_command, guild_only, rename = "faq", hide_in_help)]
pub async fn faq_prefix(
    ctx: Context<'_>,
    #[description = "Name of the faq entry"]
    #[autocomplete = "autocomplete_faq"]
    name: Option<String>,
) -> Result<(), Error> {
    if let Some(n) = name {
        faq_core(ctx, n).await?;
    } else {
        list_faqs(ctx).await?;
    }
    Ok(())
}

async fn list_faqs(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let db = &ctx.data().database;
    let Some(server) = ctx.guild_id() else {
        return Err(Box::new(CustomError::new("Could not get server ID")))
    };
    let server_id = server.get() as i64;
    let db_entries = sqlx::query!(r#"SELECT title FROM faq WHERE server_id = ?"#, server_id)
        .fetch_all(db)
        .await?;
    let faq_names = db_entries.iter().flat_map(|f| f.title.to_owned()).collect::<Vec<String>>();
    let color = Colour::GOLD;
    let embed = CreateEmbed::new()
        .title("List of FAQ tags")
        .description(faq_names.join(", "))
        .color(color);
    let builder = CreateReply::default().embed(embed);
    ctx.send(builder).await?;
    Ok(())
}

async fn faq_core(
    ctx: Context<'_>,
    name: String,
) -> Result<(), Error> {
    let db = &ctx.data().database;
    let Some(server) = ctx.guild_id() else {
        return Err(Box::new(CustomError::new("Could not get server ID")))
    };
    let server_id = server.get() as i64;
    let entry = sqlx::query!(r#"SELECT * FROM faq WHERE title = ?1 AND server_id = ?2"#, name, server_id)
        .fetch_optional(db)
        .await?;
    if let Some(e) = entry {
        let color = Colour::GOLD;
        let Some(title) = e.title else {return Err(Box::new(CustomError::new("Failed to get faq title from database")))};
        let mut embed = CreateEmbed::new()
            .title(title)
            .color(color);
        if let Some(content) = e.contents {
            embed = embed.description(content);
        }

        if let Some(img) = e.image {
            embed = embed.image(img);
        }

        let builder = CreateReply::default().embed(embed);
        ctx.send(builder).await?;
    } else {
        let response = format!("Requested faq {name} not found");
        ctx.say(response).await?;
    };
    Ok(())
}

#[allow(clippy::unused_async, clippy::cast_possible_wrap)]
async fn autocomplete_faq<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let Some(server) = ctx.guild_id() else {
        error!("Could not get server ID while autocompleting faq name"); 
        return vec![]
    };
    let server_id = server.get() as i64;
    let cache = ctx.data().faq_cache.clone();
    let faqcache = match cache.read(){
        Ok(c) => c,
        Err(e) => {
            error!{"Error acquiring cache: {e}"}
            return vec![]
        },
    };
    faqcache.iter()
        .filter(|f| f.server_id == server_id && f.name.to_lowercase().starts_with(&partial.to_lowercase()))
        .map(|f| f.name.clone())
        .collect::<Vec<String>>()
}

#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, guild_only, subcommands("new", "remove", "link"), aliases("faq-edit"))]
pub async fn faq_edit(
    _ctx: Context<'_>
) -> Result<(), Error> {
    Ok(())
}

/// Add and faq entry
#[allow(clippy::unused_async, clippy::cast_possible_wrap)]
#[poise::command(prefix_command, slash_command, guild_only, aliases("edit, add"))]
pub async fn new(
    ctx: Context<'_>,
    #[description = "Name of the faq"]
    name: String,
    #[description = "Contents of the FAQ"]
    content: String,
    #[description = "Link to an image."]
    image: String,
) -> Result<(), Error> {
    let Some(server) = ctx.guild_id() else {
        return Err(Box::new(CustomError::new("Could not get server ID")))
    };
    let server_id = server.get() as i64;
    let db = &ctx.data().database;

    if (sqlx::query!(r#"SELECT title FROM faq WHERE server_id = ?1 AND title = ?2"#, server_id, name) // Check if name already exists
        .fetch_optional(db)
        .await?).is_some() {
        // Return "faq already exists" message
        ctx.say(format!("Error: An faq entry with title {name} already exists")).await?;
    } else {
        let timestamp = ctx.created_at().timestamp();
        let author_id = ctx.author().id.get() as i64;
        sqlx::query!(r#"INSERT INTO faq (server_id, title, contents, image, edit_time, author)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#, server_id, name, content, image, timestamp, author_id)
            .execute(db)
            .await?;
        ctx.say(format!("FAQ entry {name} added to database")).await?;
    }

    Ok(())
}

/// Remove an faq entry
#[allow(clippy::unused_async, clippy::cast_possible_wrap)]
#[poise::command(prefix_command, slash_command, guild_only, aliases("delete"))]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "FAQ entry to remove"]
    #[autocomplete = "autocomplete_faq"]
    name: String
) -> Result<(), Error> {
    let Some(server) = ctx.guild_id() else {
        return Err(Box::new(CustomError::new("Could not get server ID")))
    };
    let server_id = server.get() as i64;
    let db = &ctx.data().database;
    match sqlx::query!(r#"DELETE FROM faq WHERE server_id = ?1 AND title = ?2"#, server_id, name) // Check if name already exists
        .execute(db)
        .await?
        .rows_affected() {
        0 => {
            ctx.say(format!("FAQ entry {name} does not exist in database")).await?;
        },
        _ => {
            ctx.say(format!("FAQ entry {name} removed from database")).await?;
        },
    };
    Ok(())
}

/// Link two faq titles to the same content
#[allow(clippy::unused_async, clippy::cast_possible_wrap)]
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn link(
    ctx: Context<'_>,
    #[description = "Name for link"]
    name: String,
    #[autocomplete = "autocomplete_faq"]
    #[description = "Existing FAQ entry to link to"]
    link_to: String,
) -> Result<(), Error> {
    let Some(server) = ctx.guild_id() else {
        return Err(Box::new(CustomError::new("Could not get server ID")))
    };
    let server_id = server.get() as i64;
    let db = &ctx.data().database;
    if (sqlx::query!(r#"SELECT title FROM faq WHERE server_id = ?1"#, server_id) // Check if name already exists
        .fetch_optional(db)
        .await?).is_some() {
        // Return "faq already exists" message
        ctx.say(format!("Error: An faq entry with title {name} already exists")).await?;
    } else {
        let timestamp = ctx.created_at().timestamp();
        let author_id = ctx.author().id.get() as i64;
        sqlx::query!(r#"INSERT INTO faq (server_id, title, edit_time, author, link)
        VALUES (?1, ?2, ?3, ?4, ?5)"#, server_id, name, timestamp, author_id, link_to)
        .execute(db)
        .await?;
        ctx.say(format!("FAQ link {name} added to database, linking to {link_to}")).await?;
    }
    Ok(())
}