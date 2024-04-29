use sqlx::{Pool, Sqlite};
use std::sync::{Arc, RwLock};
use poise::serenity_prelude::{CreateEmbed, Colour};
use poise::CreateReply;

use crate::{Context, Error};

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
                server_id: r.server_id.unwrap(),
                name: r.title.clone().unwrap(),
            }
        })
        .collect::<Vec<FaqCacheEntry>>();
    let mut c = cache.write().unwrap();
    *c = records;
    Ok(())
}

/// Frequently Asked Questions
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn faq(
    ctx: Context<'_>,
    #[description = "Name of the faq entry"]
    #[autocomplete = "autocomplete_faq"]
    name: String,
) -> Result<(), Error> {
    let db = &ctx.data().database;
    let entry = sqlx::query!(r#"SELECT * FROM faq WHERE title = ?1"#, name)
        .fetch_optional(db)
        .await?;
    match entry {
        Some(e) => {
            let color = Colour::GOLD;
            let mut embed = CreateEmbed::new()
                .title(e.title.clone().unwrap())
                .color(color);
            match e.contents.clone() {
                Some(c) => {
                    embed = embed.description(c);
                },
                None => {},
            };
            match e.image.clone() {
                Some(i) => {
                    embed = embed.image(i);
                }
                None => {},
            };
            let builder = CreateReply::default().embed(embed);
            ctx.send(builder).await?;
        },
        None => {
            let response = format!("Requested faq {name} not found");
            ctx.say(response).await?;
        },
    }

    Ok(())
}

async fn autocomplete_faq<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    println!("Autocompleting mod name");
    let server_id = ctx.guild_id().unwrap().get() as i64;
    let cache = ctx.data().faqcache.clone();
    let c = cache.read().unwrap().clone();
    c.iter()
        .filter(|f| f.server_id == server_id && f.name.to_lowercase().starts_with(&partial.to_lowercase()))
        .map(|f| f.name.clone())
        .collect::<Vec<String>>()
}

#[poise::command(prefix_command, slash_command, guild_only, subcommands("new", "remove", "link"))]
pub async fn faq_edit(
    _ctx: Context<'_>
) -> Result<(), Error> {
    Ok(())
}

/// Add and faq entry
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn new(
    ctx: Context<'_>,
    #[description = "Name of the faq"]
    name: String,
    #[description = "Contents of the FAQ"]
    content: String,
    #[description = "Link to an image."]
    image: String,
) -> Result<(), Error> {
    let server_id = ctx.guild_id().unwrap().get() as i64;
    let db = &ctx.data().database;
    match sqlx::query!(r#"SELECT title FROM faq WHERE server_id = ?1"#, server_id) // Check if name already exists
        .fetch_optional(db)
        .await? {
        Some(_) => {
            // Return "faq already exists" message
            ctx.say(format!("Error: An faq entry with title {name} already exists")).await?;
        },
        None => {
            let timestamp = ctx.created_at().timestamp();
            let author_id = ctx.author().id.get() as i64;
            sqlx::query!(r#"INSERT INTO faq (server_id, title, contents, image, edit_time, author) 
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#, server_id, name, content, image, timestamp, author_id)
                .execute(db)
                .await?;
            ctx.say(format!("FAQ entry {name} added to database")).await?;
        },
    }

    Ok(())
}

/// Remove an faq entry
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "FAQ entry to remove"]
    #[autocomplete = "autocomplete_faq"]
    name: String
) -> Result<(), Error> {
    let server_id = ctx.guild_id().unwrap().get() as i64;
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
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn link(
    ctx: Context<'_>,
    #[description = "Name for link"]
    name: String,
    #[autocomplete = "autocomplete_faq"]
    #[description = "Existing FAQ entry to link to"]
    link_to: String,
) -> Result<(), Error> {
    let server_id = ctx.guild_id().unwrap().get() as i64;
    let db = &ctx.data().database;
    match sqlx::query!(r#"SELECT title FROM faq WHERE server_id = ?1"#, server_id) // Check if name already exists
        .fetch_optional(db)
        .await? {
        Some(_) => {
            // Return "faq already exists" message
            ctx.say(format!("Error: An faq entry with title {name} already exists")).await?;
        },
        None => {
            let timestamp = ctx.created_at().timestamp();
            let author_id = ctx.author().id.get() as i64;
            sqlx::query!(r#"INSERT INTO faq (server_id, title, edit_time, author, link) 
                VALUES (?1, ?2, ?3, ?4, ?5)"#, server_id, name, timestamp, author_id, link_to)
                .execute(db)
                .await?;
            ctx.say(format!("FAQ link {name} added to database, linking to {link_to}")).await?;
        },
    }
    Ok(())
}