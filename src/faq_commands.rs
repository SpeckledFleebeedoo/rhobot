use log::error;
use poise::CreateReply;
use poise::ReplyHandle;
use poise::serenity_prelude as serenity;
use sqlx::{Pool, Sqlite};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::{error, fmt};

use crate::management::checks;
use crate::{
    Context, Error, SEPARATOR, database,
    database::DBFaqEntry,
    formatting_tools::DiscordFormat,
    management::{self, checks::is_mod},
    wiki_commands,
};

#[derive(Debug)]
pub enum FaqError {
    CacheError(String),
    DatabaseError(database::DatabaseError),
    ManagementError(management::ManagementError),
    SerenityError(serenity::Error),
    SerdeError(serde_json::Error),
    UTF8Error(std::str::Utf8Error),
    NotFound(String),
    NotInDatabase(String),
    WikiError(wiki_commands::WikiError, String),
    ServerNotFound,
    TitleTooLong,
    BodyTooLong,
    EmbedNotFound,
    EmbedContainsNoImage,
    AlreadyExists(String),
    NotOwner,
}

impl fmt::Display for FaqError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::CacheError(error) => f.write_str(&format!("Error acquiring cache: {error}")),
            Self::DatabaseError(error) => f.write_str(&format!("FAQ database error: {error}")),
            Self::ManagementError(error) => f.write_str(&format!("Management error: {error}")),
            Self::SerenityError(error) => f.write_str(&format!("Serenity error: {error}")),
            Self::NotFound(name) => {
                        f.write_str(&format!(
                            "Could not find {} or any similarly named tags in FAQ tags. 
                    Would you like to search [the wiki](https://wiki.factorio.com/index.php?search={})?", name.to_owned().escape_formatting(), name.replace(' ', "%20")))
                    },
            Self::NotInDatabase(name) => f.write_str(&format!("Could not get FAQ entry {name} from database")),
            Self::WikiError(error, pagename) => f.write_str(&format!("Could not find \"{pagename}\" on wiki: {error}")),
            Self::ServerNotFound => f.write_str("Could not retrieve server data."),
            Self::TitleTooLong => f.write_str("FAQ title too long (must be 256 characters or shorter)"),
            Self::BodyTooLong => f.write_str("FAQ body too long (must be 4096 characters or shorter)"),
            Self::EmbedNotFound => f.write_str("Could not create FAQ entry: embed not found"),
            Self::EmbedContainsNoImage => f.write_str("Could not create FAQ entry: image not found in embed"),
            Self::AlreadyExists(name) => f.write_str(&format!("Error: An faq entry with title {name} already exists")),
            Self::NotOwner => f.write_str("This command can only be used by the bot owner"),
            Self::SerdeError(error) => f.write_str(&format!("Error serializing or deserialziing: {error}")),
            Self::UTF8Error(error) => f.write_str(&format!("Error converting UTF8 string: {error}")),
        }
    }
}

impl error::Error for FaqError {}

impl From<database::DatabaseError> for FaqError {
    fn from(value: database::DatabaseError) -> Self {
        Self::DatabaseError(value)
    }
}

impl From<management::ManagementError> for FaqError {
    fn from(value: management::ManagementError) -> Self {
        Self::ManagementError(value)
    }
}

impl From<serenity::Error> for FaqError {
    fn from(value: serenity::Error) -> Self {
        Self::SerenityError(value)
    }
}

impl From<serde_json::Error> for FaqError {
    fn from(value: serde_json::Error) -> Self {
        Self::SerdeError(value)
    }
}

impl From<std::str::Utf8Error> for FaqError {
    fn from(value: std::str::Utf8Error) -> Self {
        Self::UTF8Error(value)
    }
}

impl From<wiki_commands::WikiError> for FaqError {
    fn from(value: wiki_commands::WikiError) -> Self {
        Self::WikiError(value, String::new())
    }
}

#[derive(Debug, Clone)]
pub struct FaqCacheEntry {
    pub server_id: i64,
    pub title: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BasicFaqEntry {
    pub title: String,
    pub contents: Option<String>,
    pub image: Option<String>,
    pub link: Option<String>,
}

pub async fn update_faq_cache(
    cache: Arc<RwLock<Vec<FaqCacheEntry>>>,
    db: &Pool<Sqlite>,
) -> Result<(), Error> {
    let records = database::get_faq_titles(db).await.map_err(FaqError::from)?;

    match cache.write() {
        Ok(mut c) => *c = records,
        Err(e) => return Err(FaqError::CacheError(e.to_string()))?,
    };
    Ok(())
}

pub fn faq() -> poise::Command<crate::Data, Error> {
    poise::Command {
        slash_action: faq_slash().slash_action,
        parameters: faq_slash().parameters,
        ..faq_prefix()
    }
}

#[allow(clippy::unused_async)]
#[poise::command(slash_command, hide_in_help, guild_only)]
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
#[poise::command(
    prefix_command,
    guild_only,
    track_edits,
    rename = "faq",
    aliases("faw", "link", "tag", "tags")
)]
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

async fn list_faqs(ctx: Context<'_>) -> Result<(), Error> {
    let db = &ctx.data().database;
    let server_id = management::get_server_id(ctx).map_err(FaqError::from)?;
    let faq_map = database::get_server_faqs(server_id, db)
        .await
        .map_err(FaqError::from)?;

    let mut faq_names: Vec<String> = Vec::new();
    for (key, links) in faq_map {
        if links.is_empty() {
            faq_names.push(key);
        } else {
            faq_names.push(format!("{key} ({})", links.join(", ")));
        }
    }

    faq_names.sort();
    let color = serenity::Colour::GOLD;
    let embed = serenity::CreateEmbed::new()
        .title("List of FAQ tags")
        .description(faq_names.join(", "))
        .color(color);
    let builder = CreateReply::default().embed(embed);
    ctx.send(builder).await.map_err(FaqError::from)?;
    Ok(())
}

async fn faq_core(ctx: Context<'_>, name: String) -> Result<(), Error> {
    let command = name.split(SEPARATOR).next().unwrap_or(&name).trim();
    let name_lc = command.capitalize();
    let db = &ctx.data().database;
    let server_id = management::get_server_id(ctx).map_err(FaqError::from)?;

    let (entry_final, close_match) = match resolve_faq_name(db, ctx, server_id, &name_lc).await {
        Ok(res) => res,
        Err(FaqError::NotFound(e)) => {
            faq_not_found(ctx, &e).await?;
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    };

    let embed = create_faq_embed(&name_lc, entry_final, close_match);
    ctx.send(embed).await.map_err(FaqError::from)?;
    Ok(())
}

// Make and send embed for faq entry
fn create_faq_embed(name: &str, faq_entry: BasicFaqEntry, close_match: bool) -> CreateReply {
    let title = if close_match {
        format!(
            r#"Could not find "{}" in FAQ tags. Did you mean "{}"?"#,
            name.escape_formatting(),
            &faq_entry.title.clone().escape_formatting()
        )
    } else {
        faq_entry.title.clone()
    };

    let mut embed = serenity::CreateEmbed::new()
        .title(title)
        .color(serenity::Colour::GOLD);
    if let Some(content) = faq_entry.contents {
        embed = embed.description(content);
    };

    if let Some(img) = faq_entry.image {
        embed = embed.image(img);
    }

    CreateReply::default().embed(embed)
}

async fn resolve_faq_name(
    db: &Pool<Sqlite>,
    ctx: Context<'_>,
    server_id: i64,
    name: &str,
) -> Result<(BasicFaqEntry, bool), FaqError> {
    // Find entry matching given `name`
    let entry_option = database::find_faq_entry_opt(db, server_id, name).await?;

    // Check if entry found
    let (entry, close_match) = if let Some(e) = entry_option {
        (e, false)
    } else {
        // If no entry found, check for near matches
        if let Some(match_name) = find_closest_faq(ctx, name, server_id)? {
            (get_faq_entry(db, server_id, &match_name).await?, true)
        } else {
            // If no near matches, return no results message
            return Err(FaqError::NotFound(name.to_string()));
        }
    };

    // If link to other entry found, get other entry
    let entry_final: BasicFaqEntry = match entry.link {
        None => entry,
        Some(entry_link) => get_faq_entry(db, server_id, &entry_link).await?,
    };
    Ok((entry_final, close_match))
}

#[allow(clippy::cast_sign_loss)]
async fn faq_not_found(ctx: Context<'_>, faq_name: &str) -> Result<(), FaqError> {
    let error = FaqError::NotFound(faq_name.to_string());
    let embed = serenity::CreateEmbed::new()
        .title("Error while executing command faq:")
        .description(format!("{error}"))
        .color(serenity::Colour::RED);
    let wiki_button = serenity::CreateButton::new("wiki_search")
        .label("Search the wiki")
        .style(serenity::ButtonStyle::Primary);
    let components = vec![serenity::CreateActionRow::Buttons(vec![wiki_button])];
    let builder = CreateReply::default()
        .embed(embed.clone())
        .components(components);
    let error_message_handle = ctx.send(builder).await.map_err(FaqError::from)?;
    let error_message = error_message_handle
        .message()
        .await
        .map_err(FaqError::from)?;
    let Some(_response) = error_message
        .await_component_interaction(ctx)
        .timeout(Duration::from_secs(120))
        .await
    else {
        let new_builder = CreateReply::default()
            .embed(embed)
            .components(Vec::default());
        match error_message_handle
            .edit(ctx, new_builder)
            .await {
                // Continue without error if message no longer exists
                Ok(()) | Err(serenity::Error::Http(_)) => return Ok(()),
                Err(e) => return Err(e.into())
            }
    };

    let wiki_embed = match wiki_commands::get_wiki_page(faq_name).await {
        Ok(w) => w,
        Err(e) => return Err(FaqError::WikiError(e, faq_name.to_string())),
    };
    let wiki_builder = CreateReply::default()
        .embed(wiki_embed)
        .components(Vec::default());
    error_message_handle.edit(ctx, wiki_builder).await?;
    Ok(())
}

async fn get_faq_entry(
    db: &Pool<Sqlite>,
    server_id: i64,
    name: &str,
) -> Result<BasicFaqEntry, FaqError> {
    database::find_faq_entry_opt(db, server_id, name)
        .await?
        .map_or_else(|| Err(FaqError::NotInDatabase(name.to_string())), Ok)
}

fn find_closest_faq(
    ctx: Context<'_>,
    name: &str,
    server_id: i64,
) -> Result<Option<String>, FaqError> {
    let cache = ctx.data().faq_cache.clone();
    let faq_cache = match cache.read() {
        Ok(c) => c,
        Err(e) => {
            return Err(FaqError::CacheError(e.to_string()));
        }
    }
    .clone();
    let server_faqs = faq_cache
        .iter()
        .filter(|f| f.server_id == server_id)
        .map(|f| f.title.as_str())
        .collect::<Vec<&str>>();
    let matches = rust_fuzzy_search::fuzzy_search_best_n(name, &server_faqs, 10);
    let best_match = matches.first();
    Ok(best_match.filter(|m| m.1 > 0.5).map(|m| m.0.to_owned()))
}

#[allow(clippy::unused_async, clippy::cast_possible_wrap)]
async fn autocomplete_faq(ctx: Context<'_>, partial: &str) -> Vec<String> {
    let Some(server) = ctx.guild_id() else {
        error!("Could not get server ID while autocompleting faq name");
        return vec![];
    };
    let server_id = server.get() as i64;
    let cache = ctx.data().faq_cache.clone();

    let mut autocomplete_vec = {
        let faqcache = match cache.read() {
            Ok(c) => c,
            Err(e) => {
                error! {"Error acquiring cache: {e}"}
                return vec![];
            }
        };
        faqcache
            .iter()
            .filter(|f| {
                f.server_id == server_id && f.title.to_lowercase().contains(&partial.to_lowercase())
            })
            .map(|f| f.title.clone())
            .collect::<Vec<String>>()
    }; // Drop faqcache variable early

    autocomplete_vec.sort_unstable();
    autocomplete_vec
}

/// Add, remove or link FAQ entries
#[allow(clippy::unused_async)]
#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    check = "is_mod",
    category = "Settings",
    subcommands("new", "remove", "link"),
    rename = "faqedit",
    aliases("faq-edit", "faq_edit"),
    subcommand_required
)]
pub async fn faq_edit(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Add an faq entry
#[allow(clippy::unused_async, clippy::cast_possible_wrap)]
#[poise::command(
    prefix_command,
    slash_command,
    guild_only,
    track_edits,
    aliases("edit", "add")
)]
pub async fn new(
    ctx: Context<'_>,
    #[description = "Name of the faq"] name: String,
    #[description = "Link to an image."] image: Option<serenity::Attachment>,
    #[description = "Contents of the FAQ"]
    #[rest]
    content: Option<String>,
) -> Result<(), Error> {
    if name.len() > 256 {
        return Err(FaqError::TitleTooLong)?;
    };
    if let Some(c) = &content {
        if c.len() > 4096 {
            return Err(FaqError::BodyTooLong)?;
        };
    };
    let name_lc = name.capitalize();
    let server = ctx.guild_id().ok_or_else(|| FaqError::ServerNotFound)?;
    let server_id = server.get() as i64;
    let db = &ctx.data().database;

    // Check if name already exists
    let pre_existing = database::find_faq_entry_opt(db, server_id, &name_lc)
        .await
        .map_err(FaqError::from)?
        .is_some();

    // If image attached, re-upload image to generate a non-ephemeral link for storage
    let (attachment_url, reply_handle) = get_attachment_url(image, ctx, &name_lc).await?;

    let timestamp = ctx.created_at().timestamp();
    let author_id = ctx.author().id.get() as i64;

    // Delete previous entry to prevent duplication
    if pre_existing {
        database::delete_faq_entry(db, server_id, &name_lc)
            .await
            .map_err(FaqError::from)?;
    };
    let faq_entry = DBFaqEntry {
        server_id,
        name: &name_lc,
        content: content.as_deref(),
        attachment_url: attachment_url.as_deref(),
        timestamp,
        author_id,
        link: None,
    };
    database::add_faq_entry(db, faq_entry)
        .await
        .map_err(FaqError::from)?;

    let title = if pre_existing {
        format!(r#"Successfully edited "{name_lc}""#)
    } else {
        format!(r#"Successfully added "{name_lc}" to database"#)
    };

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
        r.edit(ctx, builder).await.map_err(FaqError::from)?;
    } else {
        ctx.send(builder).await.map_err(FaqError::from)?;
    }
    Ok(())
}

async fn get_attachment_url<'a>(
    attachment: Option<serenity::Attachment>,
    ctx: Context<'a>,
    name: &str,
) -> Result<(Option<String>, Option<ReplyHandle<'a>>), FaqError> {
    // If image attached, re-upload image to generate a non-ephemeral link for storage
    let Some(image) = attachment else {
        return Ok((None, None));
    };

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
        return Err(FaqError::EmbedNotFound)?;
    };
    let Some(ref embed_image) = message_embed.image else {
        return Err(FaqError::EmbedContainsNoImage)?;
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
    name: String,
) -> Result<(), Error> {
    let name_lc = name.capitalize();
    let server = ctx.guild_id().ok_or_else(|| FaqError::ServerNotFound)?;
    let server_id = server.get() as i64;
    let db = &ctx.data().database;
    match database::delete_faq_entry(db, server_id, &name_lc)
        .await
        .map_err(FaqError::from)?
    {
        0 => {
            ctx.say(format!("FAQ entry {name_lc} does not exist in database"))
                .await
                .map_err(FaqError::from)?;
        }
        _ => {
            ctx.say(format!("FAQ entry {name_lc} removed from database"))
                .await
                .map_err(FaqError::from)?;
        }
    };
    Ok(())
}

/// Link two faq titles to the same content
#[allow(clippy::unused_async, clippy::cast_possible_wrap)]
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn link(
    ctx: Context<'_>,
    #[description = "Name for link"] name: String,
    #[autocomplete = "autocomplete_faq"]
    #[description = "Existing FAQ entry to link to"]
    link_to: String,
) -> Result<(), Error> {
    let name_lc = name.capitalize();
    let link_to_lc = link_to.capitalize();
    let server = ctx.guild_id().ok_or_else(|| FaqError::ServerNotFound)?;
    let server_id = server.get() as i64;
    let db = &ctx.data().database;

    // Check if name already exists
    if database::find_faq_entry_opt(db, server_id, &name_lc)
        .await
        .map_err(FaqError::from)?
        .is_some()
    {
        return Err(FaqError::AlreadyExists(name_lc))?;
    };

    let timestamp = ctx.created_at().timestamp();
    let author_id = ctx.author().id.get() as i64;

    // Find entry to link to
    let linked_entry = get_faq_entry(db, server_id, &link_to_lc).await?;
    let link_no_chain = linked_entry.link.map_or(link_to_lc, |link| link);
    let faq_entry = DBFaqEntry {
        server_id,
        name: &name_lc,
        content: None,
        attachment_url: None,
        timestamp,
        author_id,
        link: Some(&link_no_chain),
    };
    database::add_faq_entry(db, faq_entry)
        .await
        .map_err(FaqError::from)?;
    ctx.say(format!(
        "FAQ link {name_lc} added to database, linking to {link_no_chain}"
    ))
    .await
    .map_err(FaqError::from)?;
    Ok(())
}

/// Drop all FAQ entries for this server
#[allow(clippy::unused_async)]
#[poise::command(
    slash_command,
    guild_only,
    owners_only,
    hide_in_help,
    ephemeral,
    category = "Management"
)]
pub async fn drop_faqs(ctx: Context<'_>) -> Result<(), Error> {
    let db = &ctx.data().database;
    let server_id = management::get_server_id(ctx).map_err(FaqError::from)?;
    let button_yes = serenity::CreateButton::new("Yes")
        .label("Yes")
        .style(serenity::ButtonStyle::Danger);
    let button_no = serenity::CreateButton::new("No")
        .label("No")
        .style(serenity::ButtonStyle::Primary);
    let components = vec![serenity::CreateActionRow::Buttons(vec![
        button_yes, button_no,
    ])];
    let confirmation = ctx.send(
        CreateReply::default()
            .content("Are you sure you want to drop the FAQ database for this server? \n**THIS ACTION CANNOT BE REVERTED**")
            .components(components)
        ).await.map_err(FaqError::from)?;
    let confirmation_message = confirmation.message().await.map_err(FaqError::from)?;

    let Some(response) = confirmation_message
        .await_component_interaction(ctx)
        .timeout(Duration::from_secs(60))
        .await
    else {
        let new_message = CreateReply::default()
            .content("Timed out")
            .components(Vec::default());
        confirmation
            .edit(ctx, new_message)
            .await
            .map_err(FaqError::from)?;
        return Ok(());
    };

    if checks::is_owner(ctx, response.user)
        .await
        .map_err(FaqError::from)?
    {
        if response.data.custom_id == "Yes" {
            let faq_str = create_faq_dump(server_id, db).await?;
            let faq_file = serenity::CreateAttachment::bytes(
                faq_str,
                format!(
                    "FAQ_dump_{}_{}.json",
                    server_id,
                    ctx.created_at().timestamp()
                ),
            );
            let builder = CreateReply::default()
                .content("Created dump of FAQ contents:")
                .attachment(faq_file);
            ctx.send(builder).await.map_err(FaqError::from)?;
            database::clear_server_faq(db, server_id)
                .await
                .map_err(FaqError::from)?;
            let new_message = CreateReply::default()
                .content("All FAQ entries for this server deleted")
                .components(Vec::default());
            confirmation
                .edit(ctx, new_message)
                .await
                .map_err(FaqError::from)?;
        } else {
            let new_message = CreateReply::default()
                .content("No changes made")
                .components(Vec::default());
            confirmation
                .edit(ctx, new_message)
                .await
                .map_err(FaqError::from)?;
        }
    } else {
        return Err(FaqError::NotOwner)?;
    }

    Ok(())
}

async fn create_faq_dump(server_id: i64, db: &Pool<Sqlite>) -> Result<String, Error> {
    let server_faqs = database::get_server_faq_dump(db, server_id)
        .await
        .map_err(FaqError::from)?;

    let faq_json = serde_json::to_string(&server_faqs).map_err(FaqError::from)?;

    Ok(faq_json)
}

/// Export all server FAQs to a json file
#[poise::command(
    slash_command,
    guild_only,
    owners_only,
    hide_in_help,
    category = "Management"
)]
pub async fn export_faqs(ctx: Context<'_>) -> Result<(), Error> {
    let db = &ctx.data().database;
    let server_id = management::get_server_id(ctx).map_err(FaqError::from)?;
    let faq_str = create_faq_dump(server_id, db).await?;
    let faq_file = serenity::CreateAttachment::bytes(
        faq_str,
        format!(
            "FAQ_dump_{}_{}.json",
            server_id,
            ctx.created_at()
                .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        ),
    );
    let builder = CreateReply::default()
        .content("Created dump of FAQ contents:")
        .attachment(faq_file);
    ctx.send(builder).await.map_err(FaqError::from)?;
    Ok(())
}

/// Import all FAQs from a json file. May lead to duplicate entries.
#[allow(clippy::cast_possible_wrap)]
#[poise::command(
    slash_command,
    guild_only,
    owners_only,
    hide_in_help,
    category = "Management"
)]
pub async fn import_faqs(ctx: Context<'_>, faq_json: serenity::Attachment) -> Result<(), Error> {
    let server_id = management::get_server_id(ctx).map_err(FaqError::from)?;
    let content = faq_json.download().await.map_err(FaqError::from)?;
    let file_str = std::str::from_utf8(&content).map_err(FaqError::from)?;
    let faqs: Vec<BasicFaqEntry> = serde_json::from_str(file_str).map_err(FaqError::from)?;
    let db = &ctx.data().database;
    let timestamp = ctx.created_at().timestamp();
    let author = ctx.author().id.get() as i64;
    for faq in faqs {
        let db_faq_entry = database::DBFaqEntry {
            server_id,
            name: &faq.title,
            content: faq.contents.as_deref(),
            attachment_url: faq.image.as_deref(),
            timestamp,
            author_id: author,
            link: faq.link.as_deref(),
        };
        database::add_faq_entry(db, db_faq_entry)
            .await
            .map_err(FaqError::from)?;
    }
    ctx.say("Successfully imported all FAQ entries")
        .await
        .map_err(FaqError::from)?;
    Ok(())
}
