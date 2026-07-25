use ::serenity::all::prelude::CacheHttp;
use log::{error, info};
use poise::CreateReply;
use poise::serenity_prelude as serenity;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;

use crate::{Context, Data, Error, database, mods::commands, wiki_commands};

pub struct CustomEventHandler {
    pub data: Arc<Data>,
}

impl CustomEventHandler {
    pub const fn new(data: Arc<Data>) -> Self {
        Self { data }
    }
}

#[serenity::async_trait]
impl serenity::EventHandler for CustomEventHandler {
    async fn dispatch(&self, ctx: &serenity::Context, event: &serenity::FullEvent) {
        match event {
            serenity::FullEvent::GuildDelete { incomplete, .. } => {
                if !incomplete.unavailable {
                    let _ = on_guild_leave(incomplete.id, &self.data.database).await;
                }
            }
            serenity::FullEvent::Message { new_message, .. } => {
                let _ = on_message(ctx.clone(), new_message, &self.data).await;
            }
            serenity::FullEvent::MessageDelete {
                channel_id,
                deleted_message_id,
                ..
            } => {
                let _ = on_message_delete(ctx.clone(), channel_id, deleted_message_id, &self.data)
                    .await;
            }
            serenity::FullEvent::MessageUpdate { event, .. } => {
                let _ = on_message_edit(ctx.clone(), event, &self.data).await;
            }
            serenity::FullEvent::ReactionAdd { add_reaction, .. } => {
                let _ = on_react_added(ctx.clone(), add_reaction).await;
            }
            serenity::FullEvent::Ready { data_about_bot, .. } => {
                println!("Logged in as {}", data_about_bot.user.name);
                log::info!("Logged in as {}", data_about_bot.user.name);
            }
            _ => (),
        }
    }
}

pub async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            error.log();
            let _ = send_custom_error_message(ctx, &format!("{error}")).await;
        }
        poise::FrameworkError::CommandCheckFailed { ctx, .. } => {
            let _ = send_custom_error_message(
                ctx,
                "I'm sorry, Dave. I'm afraid I can't do that\nInvalid permissions",
            )
            .await;
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                error!("Error while handling error: {e}");
            }
        }
    }
}

async fn send_custom_error_message(ctx: Context<'_>, msg: &str) -> Result<(), Error> {
    let embed = serenity::CreateEmbed::new()
        .title(format!(
            "Error while executing command {}:",
            ctx.command().name
        ))
        .description(msg)
        .color(serenity::Colour::RED);
    let builder = CreateReply::default()
        .embed(embed)
        .reply(true)
        .allowed_mentions(serenity::CreateAllowedMentions::default());
    ctx.send(builder).await?;
    Ok(())
}

#[allow(clippy::unnecessary_unwrap)]
pub async fn on_message(
    ctx: serenity::Context,
    msg: &serenity::Message,
    data: &Data,
) -> Result<(), Error> {
    if msg.author.bot() {
        return Ok(());
    }
    let wikisearch = message_prompt_search(&msg.content, '[', ']');
    let modsearch = message_prompt_search(&msg.content, '>', '<');
    if !modsearch.is_empty() || !wikisearch.is_empty() {
        if let Some(response) =
            send_inline_search_response(&ctx, msg, data, modsearch, wikisearch).await?
        {
            data.inline_command_log.insert(
                msg.id,
                (msg.channel_id, response, tokio::time::Instant::now()),
            );
        }
        return Ok(());
    }
    Ok(())
}

pub async fn on_message_edit(
    ctx: serenity::Context,
    msg: &serenity::MessageUpdateEvent,
    data: &Data,
) -> Result<(), Error> {
    if !data.inline_command_log.contains_key(&msg.message.id) {
        return Ok(());
    }
    let (channel_id, message_id, _) = *data.inline_command_log.get(&msg.message.id).unwrap();
    let message_content = &msg.message.content;
    let wikisearch = message_prompt_search(message_content, '[', ']');
    let modsearch = message_prompt_search(message_content, '>', '<');
    if !modsearch.is_empty() || !wikisearch.is_empty() {
        update_inline_search_response(&ctx, data, channel_id, message_id, modsearch, wikisearch)
            .await?;
        return Ok(());
    }

    // No command present in message anymore -> delete response
    let message = channel_id.message(&ctx, message_id).await?;
    message.delete(&ctx.http, None).await?;
    data.inline_command_log.remove(&msg.message.id);

    Ok(())
}

pub async fn on_message_delete(
    ctx: serenity::Context,
    channel_id: &serenity::all::GenericChannelId,
    deleted_message_id: &serenity::all::MessageId,
    data: &Data,
) -> Result<(), Error> {
    if !data.inline_command_log.contains_key(deleted_message_id) {
        return Ok(());
    }
    let (_, message_id, _) = *data.inline_command_log.get(deleted_message_id).unwrap();
    let message = channel_id.message(&ctx, message_id).await?;
    message.delete(&ctx.http, None).await?;
    data.inline_command_log.remove(deleted_message_id);

    Ok(())
}

pub async fn on_react_added(
    ctx: serenity::Context,
    reaction: &serenity::Reaction,
) -> Result<(), Error> {
    // Check if emoji is correct
    if reaction.emoji != serenity::ReactionType::from('❌') {
        return Ok(());
    }
    // Check if message is from bot
    if reaction.message_author_id != Some(ctx.cache.current_user().id) {
        return Ok(());
    }

    let reacted_message = reaction.message(&ctx.http).await?;

    // Check if user who reacted is the same user who invoked the bot
    if let Some(ref interaction) = reacted_message.interaction {
        if Some(interaction.user.id) != reaction.user_id {
            return Ok(());
        }
    } else {
        let Some(ref referenced_message) = reacted_message.referenced_message else {
            return Ok(());
        };
        if Some(referenced_message.author.id) != reaction.user_id {
            return Ok(());
        }
    }

    reacted_message
        .delete(
            &ctx.http,
            Some("Cleaning up own message in response to ❌ react"),
        )
        .await?;
    Ok(())
}

fn message_prompt_search(
    message_content: &str,
    opening_char: char,
    closing_char: char,
) -> Vec<String> {
    let mut in_code_block = false;
    let mut blockquote_depth = 0;
    let mut filtered_message = String::new();
    for event in pulldown_cmark::Parser::new(message_content) {
        match event {
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::CodeBlock(_)) => {
                in_code_block = true;
            }
            pulldown_cmark::Event::End(pulldown_cmark::TagEnd::CodeBlock) => {
                in_code_block = false;
            }
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::BlockQuote(None)) => {
                filtered_message.push('>');
                blockquote_depth += 1;
            }
            pulldown_cmark::Event::SoftBreak => {
                for _ in 0..blockquote_depth {
                    filtered_message.push('>');
                }
            }
            pulldown_cmark::Event::Text(pulldown_cmark::CowStr::Borrowed(text))
                if !in_code_block =>
            {
                filtered_message.push_str(text);
            }
            pulldown_cmark::Event::End(pulldown_cmark::TagEnd::Paragraph) => {
                filtered_message.push('\n');
            }
            _ => (),
        }
    }

    let char_vec = filtered_message.chars().collect::<Vec<char>>();
    let mut start_index: Option<usize> = None;
    let mut results = Vec::new();

    let mut start_counter = 0;
    let mut end_counter = 0;

    for i in 0..char_vec.len() {
        let current_char = char_vec[i];
        if current_char == opening_char {
            start_counter += 1;
            if start_counter == 2 {
                start_index = Some(i + 1);
            } else if start_counter > 2 {
                start_index = None;
            }
            end_counter = 0;
        } else if current_char == closing_char {
            end_counter += 1;
            start_counter = 0;
        }

        if let Some(s) = start_index
            && end_counter == 2
        {
            let modname = filtered_message[s..i - 1].to_string();
            if !modname.is_empty() {
                results.push(modname);
            }
            start_index = None;
        }
    }
    results
}

async fn send_inline_search_response(
    ctx: &serenity::Context,
    msg: &serenity::Message,
    data: &Data,
    modnames: Vec<String>,
    wikinames: Vec<String>,
) -> Result<Option<serenity::MessageId>, Error> {
    let mut embeds: Vec<serenity::CreateEmbed> = Vec::new();
    for modname in &modnames {
        if let Ok(embed) = commands::mod_search(modname.to_owned(), true, data).await {
            embeds.push(embed);
        }
    }
    for wikiname in &wikinames {
        if let Some(search_result) = search_wiki_page_name(wikiname).await? {
            embeds.push(wiki_commands::get_wiki_page(search_result).await?);
        }
    }
    if embeds.is_empty() {
        Ok(None)
    } else {
        let builder: serenity::CreateMessage = serenity::CreateMessage::new().add_embeds(embeds);
        let response = msg.channel_id.send_message(ctx.http(), builder).await?;
        Ok(Some(response.id))
    }
}

async fn update_inline_search_response(
    ctx: &serenity::Context,
    data: &Data,
    channel_id: serenity::GenericChannelId,
    message_id: serenity::MessageId,
    modnames: Vec<String>,
    wikinames: Vec<String>,
) -> Result<(), Error> {
    let mut embeds: Vec<serenity::CreateEmbed> = Vec::new();
    for modname in modnames {
        if let Ok(embed) = commands::mod_search(modname, true, data).await {
            embeds.push(embed);
        }
    }
    for wikiname in wikinames {
        if let Some(search_result) = search_wiki_page_name(&wikiname).await? {
            embeds.push(wiki_commands::get_wiki_page(search_result).await?);
        }
    }
    if !embeds.is_empty() {
        let builder: serenity::EditMessage = serenity::EditMessage::new().add_embeds(embeds);
        channel_id
            .edit_message(&ctx.http, message_id, builder)
            .await?;
    }
    Ok(())
}

async fn search_wiki_page_name(name: &str) -> Result<Option<String>, Error> {
    let results = wiki_commands::opensearch_mediawiki(name).await?;
    let Some(res) = results.first() else {
        return Ok(None);
    };
    Ok(Some(res.clone()))
}

pub fn clean_inline_command_log(
    command_log: &Arc<
        dashmap::DashMap<
            serenity::MessageId,
            (
                serenity::GenericChannelId,
                serenity::MessageId,
                tokio::time::Instant,
            ),
        >,
    >,
) {
    let cutoff_time = tokio::time::Instant::now() - tokio::time::Duration::from_hours(1);
    command_log.retain(|_, (_, _, t)| *t >= cutoff_time);
}

#[allow(clippy::cast_possible_wrap)]
pub async fn on_guild_leave(id: serenity::GuildId, db: &Pool<Sqlite>) -> Result<(), Error> {
    let server_id = id.get() as i64;
    database::clear_server_data(server_id, db).await?;
    info!("Left guild {server_id}");
    Ok(())
}
