use std::collections::HashMap;
use std::time::Duration;
use poise::ReplyHandle;
use sqlx::{Pool, Sqlite};
use std::sync::{Arc, RwLock};
use poise::serenity_prelude as serenity;
use poise::CreateReply;
use log::error;

use crate::{
    Context, 
    custom_errors::CustomError, 
    Error, 
    fun_commands, 
    management::{self, checks::is_mod},
    SEPARATOR, 
    formatting_tools::DiscordFormat, 
};

#[derive(Debug, Clone)]
pub struct FaqCacheEntry {
    server_id: i64,
    title: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct FaqEntry {
    title: String,
    contents: Option<String>,
    image: Option<String>,
    link: Option<String>,
}

pub async fn update_faq_cache(
    cache: Arc<RwLock<Vec<FaqCacheEntry>>>,
    db: Pool<Sqlite>
) -> Result<(), Error> {
    let records = sqlx::query_as!(FaqCacheEntry, r#"SELECT server_id, title FROM faq"#)
        .fetch_all(&db)
        .await?;

    match cache.write() {
        Ok(mut c) => {*c = records},
        Err(e) => {
            return Err(Box::new(CustomError::new(&format!("Error acquiring cache: {e}"))));
        },
    };
    Ok(())
}

pub fn faq() -> poise::Command<crate::Data, Box<dyn std::error::Error + Send + Sync>> {
    poise::Command {
        slash_action: faq_slash().slash_action,
        parameters: faq_slash().parameters,
        ..faq_prefix()
    }
}

/// Frequently Asked Questios
#[allow(clippy::unused_async)]
#[poise::command(slash_command, guild_only)]
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
#[allow(clippy::unused_async)]
#[poise::command(prefix_command, guild_only, hide_in_help, track_edits, rename = "faq", aliases("faw", "link", "tag", "tags"))]
pub async fn faq_prefix(
    ctx: Context<'_>,
    #[description = "Name of the faq entry"]
    #[rest]
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
    let server_id = management::get_server_id(ctx)?;
    let db_entries = sqlx::query!(r#"SELECT title, link FROM faq WHERE server_id = $1"#, server_id)
        .fetch_all(db)
        .await?;
    let mut faq_map: HashMap<String, Vec<String>> = HashMap::new();
    let base_faqs: Vec<String> = db_entries.iter().filter(|f| f.link.is_none()).map(|f| f.title.clone()).collect();
    let link_faqs: Vec<(String, String)> = db_entries.iter()
        .filter_map(|f| f.link
            .clone()
            .map(|l| (f.title.clone(), l))
        )
        .collect();
    
    for entry in &base_faqs {
        faq_map.insert(entry.clone(), Vec::new());
    }
    for (faq, link) in link_faqs {
        if let Some(map) = faq_map.get_mut(&link) {
            map.push(faq);
        }
    }
    let mut faq_names = Vec::new();
    for key in base_faqs {
        #[allow(clippy::option_if_let_else)]
        let link_list = match faq_map.get(&key) {
            Some(l) => l,
            None => &Vec::new(),
        };
        if link_list.is_empty() {
            faq_names.push(key);
        } else {
            let links = link_list.join(", ");
            faq_names.push(format!("{key} ({links})"));
        }
    }

    faq_names.sort();
    let color = serenity::Colour::GOLD;
    let embed = serenity::CreateEmbed::new()
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
    let command = name.split(SEPARATOR).next().unwrap_or(&name).trim();
    let name_lc = command.capitalize();
    let db = &ctx.data().database;
    let server_id = management::get_server_id(ctx)?;

    let (entry_final, close_match) = resolve_faq_name(db, ctx, server_id, &name_lc).await?;

    let embed = create_faq_embed(&name_lc, entry_final, close_match);
    ctx.send(embed).await?;
    Ok(())
}

// Make and send embed for faq entry
fn create_faq_embed(name: &str, faq_entry: FaqEntry, close_match: bool) -> CreateReply {
    let title = if close_match {
        format!(r#"Could not find "{}" in FAQ tags. Did you mean "{}"?"#, name.escape_formatting(), &faq_entry.title.clone().escape_formatting())
    } else {
        faq_entry.title.clone()
    };

    let mut embed = serenity::CreateEmbed::new()
        .title(title)
        .color(serenity::Colour::GOLD);
    if let Some(content) = faq_entry.contents {
        let seconds_until_release = fun_commands::time_until_release();
        if faq_entry.title == "Expansion" && seconds_until_release >= 0 {
            let (duration, unit) = match seconds_until_release {
                86400..=i64::MAX => (seconds_until_release / 86400, "days"),
                3600..86400 => (seconds_until_release / 3600, "hours"),
                60..3600 => (seconds_until_release/60, "minutes"),
                _ => (seconds_until_release, "seconds")
            };
            embed = embed.description(format!("{content}\n\nCountdown: {duration} {unit} until release"));
        } else {
            embed = embed.description(content);
        }
    };

    if let Some(img) = faq_entry.image {
        embed = embed.image(img);
    }

    CreateReply::default().embed(embed)
}

async fn resolve_faq_name(db: &Pool<Sqlite>, ctx: Context<'_>, server_id: i64, name: &str) -> Result<(FaqEntry, bool), Error> {
    // Find entry matching given `name`
    let entry_option = find_faq_entry_opt(db, server_id, name).await?;

    // Check if entry found
    let (entry, close_match) = if let Some(e) = entry_option 
    {
        (e, false) 
    } else {
        // If no entry found, check for near matches
        if let Some(match_name) = find_closest_faq(ctx, name, server_id)? {
            (get_faq_entry(db, server_id, &match_name).await?, true)
        } else {
            // If no near matches, return no results message
            let errmsg = format!(
                "Could not find {} or any similarly tags in FAQ tags. 
                Would you like to search [the wiki](https://wiki.factorio.com/index.php?search={})?", name.to_owned().escape_formatting(), name.replace(' ', "%20"));
            return Err(Box::new(CustomError::new(&errmsg)));
        }
    };

    // If link to other entry found, get other entry
    let entry_final: FaqEntry = match entry.link {
        None => entry,
        Some(entry_link) => {
            get_faq_entry(db, server_id, &entry_link).await?
        }
    };
    Ok((entry_final, close_match))
}

async fn get_faq_entry(db: &Pool<Sqlite>, server_id: i64, name: &str) -> Result<FaqEntry, Error> {
    Ok(find_faq_entry_opt(db, server_id, name)
        .await?
        .map_or_else(|| Err(Box::new(CustomError::new(&format!("Could not get FAQ entry {name} from database")))), Ok)?
    )
}

async fn find_faq_entry_opt(db: &Pool<Sqlite>, server_id: i64, name: &str) -> Result<Option<FaqEntry>, Error> {
    Ok(sqlx::query_as!(FaqEntry, 
        r#"SELECT title, contents, image, link FROM faq WHERE server_id = $1 AND title = $2"#, server_id, name)
        .fetch_optional(db)
        .await?)
}

fn find_closest_faq(ctx: Context<'_>, name: &str, server_id: i64) -> Result<Option<String>, Error> {
    let cache = ctx.data().faq_cache.clone();
    let faq_cache = match cache.read() {
        Ok(c) => c,
        Err(e) => {
            return Err(Box::new(CustomError::new(&format!("Error acquiring cache: {e}"))));
        },
    }.clone();
    let server_faqs = faq_cache.iter().filter(|f| f.server_id == server_id).map(|f| f.title.as_str()).collect::<Vec<&str>>();
    let matches = rust_fuzzy_search::fuzzy_search_best_n(name, &server_faqs, 10);
    let best_match = matches.first();
    Ok(best_match
        .filter(|m| m.1 > 0.5)
        .map(|m| m.0.to_owned())
    )
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
        .filter(|f| f.server_id == server_id && f.title.to_lowercase().starts_with(&partial.to_lowercase()))
        .map(|f| f.title.clone())
        .collect::<Vec<String>>()
}

/// Add, remove or link FAQ entries
#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, guild_only, check="is_mod", category="Settings", subcommands("new", "remove", "link"), rename = "faqedit", aliases("faq-edit", "faq_edit"), subcommand_required)]
pub async fn faq_edit(
    _ctx: Context<'_>
) -> Result<(), Error> {
    Ok(())
}

/// Add an faq entry
#[allow(clippy::unused_async, clippy::cast_possible_wrap)]
#[poise::command(prefix_command, slash_command, guild_only, track_edits, aliases("edit", "add"))]
pub async fn new(
    ctx: Context<'_>,
    #[description = "Name of the faq"]
    name: String,
    #[description = "Link to an image."]
    image: Option<serenity::Attachment>,
    #[description = "Contents of the FAQ"]
    #[rest]
    content: Option<String>,
) -> Result<(), Error> {
    if name.len() > 256 {
        return Err(Box::new(CustomError::new("FAQ title too long (must be 256 characters or shorter)")));
    };
    if let Some(c) = &content {
        if c.len() > 4096 {
            return Err(Box::new(CustomError::new("FAQ body too long (must be 4096 characters or shorter)")));
        };
    };
    let name_lc = name.capitalize();
    let Some(server) = ctx.guild_id() else {
        return Err(Box::new(CustomError::new("Could not get server ID")))
    };
    let server_id = server.get() as i64;
    let db = &ctx.data().database;

    // Check if name already exists
    let pre_existing = find_faq_entry_opt(db, server_id, &name_lc).await?.is_some();

    // If image attached, re-upload image to generate a non-ephemeral link for storage
    let (attachment_url, reply_handle) = get_attachment_url(image, ctx, &name_lc).await?;

    let timestamp = ctx.created_at().timestamp();
    let author_id = ctx.author().id.get() as i64;

    // Delete previous entry to prevent duplication
    if pre_existing {
        delete_faq_entry(db, server_id, &name_lc).await?;
    };
    sqlx::query!(r#"INSERT INTO faq (server_id, title, contents, image, edit_time, author)
    VALUES ($1, $2, $3, $4, $5, $6)"#, server_id, name_lc, content, attachment_url, timestamp, author_id)
        .execute(db)
        .await?;

    let title = if pre_existing {format!(r#"Successfully edited "{name_lc}""#)}
        else {format!(r#"Successfully added "{name_lc}" to database"#)};

    let mut embed = serenity::CreateEmbed::new()
        .title(title)
        .colour(serenity::Colour::DARK_GREEN);
    if let Some(c) = content {
        embed = embed.description(c);
    }
    if let Some(url) = attachment_url {
        embed = embed.image(url);
    }
    let builder = CreateReply::default().embed(embed);
    if let Some(r) = reply_handle {
        r.edit(ctx, builder).await?;
    } else {
        ctx.send(builder).await?;
    }
    Ok(())
}

async fn delete_faq_entry(db: &Pool<Sqlite>, server_id: i64, name: &str) -> Result<u64, Error> {
    Ok(sqlx::query!(r#"DELETE FROM faq WHERE server_id = $1 AND title = $2"#, server_id, name)
        .execute(db)
        .await?
        .rows_affected())
}

async fn get_attachment_url<'a>(attachment: Option<serenity::Attachment>, ctx: Context<'a>, name: &str) -> Result<(Option<String>, Option<ReplyHandle<'a>>), Error> {
    // If image attached, re-upload image to generate a non-ephemeral link for storage
    let Some(image) = attachment else {return Ok((None, None))};

    if !image.ephemeral {
        return Ok((Some(image.url.clone()), None));
    }

    let attachment = serenity::CreateAttachment::url(ctx.http(), &image.url).await?;
    let embed = serenity::CreateEmbed::new()
        .title(format!("Adding FAQ entry: {name}"))
        .description("Uploading image to Discord...")
        .colour(serenity::Colour::DARK_GREEN)
        .attachment(image.filename);
    let builder = CreateReply::default().attachment(attachment).embed(embed);
    let reply = ctx.send(builder).await?;
    let message = reply.message().await?;
    
    let Some(message_embed) = message.embeds.first() else {
        return Err(Box::new(CustomError::new("Could not create FAQ entry: embed not found")))
    };
    let Some(ref embed_image) = message_embed.image else {
        return Err(Box::new(CustomError::new("Could not create FAQ entry: image not found in embed")))
    };
    Ok((Some(embed_image.url.clone()), Some(reply.clone())))
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
    let name_lc = name.capitalize();
    let Some(server) = ctx.guild_id() else {
        return Err(Box::new(CustomError::new("Could not get server ID")))
    };
    let server_id = server.get() as i64;
    let db = &ctx.data().database;
    match delete_faq_entry(db, server_id, &name_lc).await? {
        0 => {
            ctx.say(format!("FAQ entry {name_lc} does not exist in database")).await?;
        },
        _ => {
            ctx.say(format!("FAQ entry {name_lc} removed from database")).await?;
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
    let name_lc = name.capitalize();
    let link_to_lc = link_to.capitalize();
    let Some(server) = ctx.guild_id() else {
        return Err(Box::new(CustomError::new("Could not get server ID")))
    };
    let server_id = server.get() as i64;
    let db = &ctx.data().database;

    // Check if name already exists
    if find_faq_entry_opt(db, server_id, &name_lc)
        .await?
        .is_some() 
    {
        return Err(Box::new(CustomError::new(&format!("Error: An faq entry with title {name_lc} already exists"))));
    };
    
    let timestamp = ctx.created_at().timestamp();
    let author_id = ctx.author().id.get() as i64;
    
    // Find entry to link to
    let linked_entry = get_faq_entry(db, server_id, &link_to_lc).await?;
    let link_no_chain = linked_entry.link.map_or(link_to_lc, |link| link);
    insert_faq_link(db, server_id, &name_lc, &link_no_chain, author_id, timestamp).await?;
    ctx.say(format!("FAQ link {name_lc} added to database, linking to {link_no_chain}")).await?;
    Ok(())
}

async fn insert_faq_link(
    db: &Pool<Sqlite>, 
    server_id: i64, 
    name: &str, 
    link: &str, 
    author_id: i64, 
    timestamp: i64
) -> Result<(), Error> {
    sqlx::query!(r#"INSERT INTO faq (server_id, title, edit_time, author, link)
        VALUES ($1, $2, $3, $4, $5)"#, server_id, name, timestamp, author_id, link)
        .execute(db)
        .await?;
    Ok(())
}

/// Drop all FAQ entries for this server
#[allow(clippy::unused_async)]
#[poise::command(slash_command, guild_only, owners_only, hide_in_help, ephemeral, category="Management")]
pub async fn drop_faqs(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let db = &ctx.data().database;
    let server_id = management::get_server_id(ctx)?;
    let button_yes = serenity::CreateButton::new("Yes").label("Yes").style(serenity::ButtonStyle::Danger);
    let button_no = serenity::CreateButton::new("No").label("No").style(serenity::ButtonStyle::Primary);
    let components = vec![serenity::CreateActionRow::Buttons(vec![button_yes, button_no])];
    let confirmation = ctx.send(
        CreateReply::default()
            .content("Are you sure you want to drop the FAQ database for this server? \n**THIS ACTION CANNOT BE REVERTED**")
            .components(components)
        ).await?;
    let confirmation_message = confirmation
        .message()
        .await?;

    let Some(response) = confirmation_message
        .await_component_interaction(ctx)
        .timeout(Duration::from_secs(60))
        .await 
    else {
        let new_message = CreateReply::default()
            .content("Timed out")
            .components(Vec::default());
        confirmation.edit(ctx, new_message).await?;
        return Ok(());
    };

    let Some(owner) = ctx.http().get_current_application_info().await?.owner else {
        return Err(Box::new(CustomError::new("Failed to verify if user is owner")))
    };
    if response.user == owner {
        if response.data.custom_id == "Yes" {
            let faq_str = create_faq_dump(server_id, db).await?;
            let faq_file = serenity::CreateAttachment::bytes(faq_str, format!("FAQ_dump_{}_{}.json", server_id, ctx.created_at().timestamp()));
            let builder = CreateReply::default()
                .content("Created dump of FAQ contents:")
                .attachment(faq_file);
            ctx.send(builder).await?;
            sqlx::query!(r#"DELETE FROM faq WHERE server_id = $1"#, server_id)
                .execute(db)
                .await?;
            let new_message = CreateReply::default()
                .content("All FAQ entries for this server deleted")
                .components(Vec::default());
            confirmation.edit(ctx, new_message).await?;
        }
        else {
            let new_message = CreateReply::default()
                .content("No changes made")
                .components(Vec::default());
            confirmation.edit(ctx, new_message).await?;
        }
    } else {
        return Err(Box::new(CustomError::new("This command can only be used by the bot owner")))
    }
    
    Ok(())
}

async fn create_faq_dump(server_id: i64, db: &Pool<Sqlite>) -> Result<String, Error> {
    let server_faqs = sqlx::query_as!(FaqEntry, r#"SELECT title, contents, image, link FROM faq WHERE server_id = $1"#, server_id)
        .fetch_all(db)
        .await?;

    let faq_json = serde_json::to_string(&server_faqs)?;

    Ok(faq_json)
}

/// Export all server FAQs to a json file
#[poise::command(slash_command, guild_only, owners_only, hide_in_help, category="Management")]
pub async fn export_faqs(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let db = &ctx.data().database;
    let server_id = management::get_server_id(ctx)?;
    let faq_str = create_faq_dump(server_id, db).await?;
    let faq_file = serenity::CreateAttachment::bytes(faq_str, format!("FAQ_dump_{}_{}.json", server_id, ctx.created_at().timestamp()));
    let builder = CreateReply::default()
        .content("Created dump of FAQ contents:")
        .attachment(faq_file);
    ctx.send(builder).await?;
    Ok(())
}


/// Import all FAQs from a json file. May lead to duplicate entries.
#[allow(clippy::cast_possible_wrap)]
#[poise::command(slash_command, guild_only, owners_only, hide_in_help, category="Management")]
pub async fn import_faqs(
    ctx: Context<'_>,
    faq_json: serenity::Attachment,
) -> Result<(), Error> {
    let server_id = management::get_server_id(ctx)?;
    // let timestamp = ctx.created_at().timestamp();
    let content = faq_json.download().await?;
    let file_str = std::str::from_utf8(&content)?;
    let faqs: Vec<FaqEntry> = serde_json::from_str(file_str)?;
    let db = &ctx.data().database;
    let timestamp = ctx.created_at().timestamp();
    let author = ctx.author().id.get() as i64;
    for faq in faqs {
        sqlx::query!(r#"
            INSERT INTO faq (server_id, title, contents, image, edit_time, author, link) 
            VALUES ($1, $2, $3, $4, $5, $6, $7)"#, 
            server_id,
            faq.title,
            faq.contents,
            faq.image,
            timestamp,
            author,
            faq.link
        )
            .execute(db)
            .await?;
    };
    ctx.say("Successfully imported all FAQ entries").await?;
    Ok(())
}