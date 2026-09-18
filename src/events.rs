use ::serenity::all::prelude::CacheHttp;
use log::{error, info};
use poise::CreateReply;
use poise::serenity_prelude as serenity;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;

use crate::faq_commands;
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
                let _ = on_message(ctx.clone(), new_message, &self.data, None).await;
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
                let _ = on_message_edit(ctx.clone(), &event.message, &self.data).await;
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

#[derive(Debug, Clone)]
pub struct BotMessageInfo {
    server_id: serenity::GuildId,
    channel_id: serenity::GenericChannelId,
    message_id: serenity::MessageId,
    timestamp: tokio::time::Instant,
}

impl BotMessageInfo {
    async fn get_message(
        &self,
        ctx: &serenity::Context,
    ) -> Result<serenity::Message, serenity::Error> {
        self.channel_id.message(&ctx, self.message_id).await
    }
}

#[derive(Debug, Clone)]
enum InlineCommandType {
    Wiki,
    Mod,
    Faq,
}

impl InlineCommandType {
    const fn start(&self) -> char {
        match self {
            Self::Wiki => '[',
            Self::Mod => '>',
            Self::Faq => '{',
        }
    }
    const fn end(&self) -> char {
        match self {
            Self::Wiki => ']',
            Self::Mod => '<',
            Self::Faq => '}',
        }
    }
}

#[derive(Debug, Clone)]
struct InlineCommand {
    name: String,
    kind: InlineCommandType,
}

#[poise::command(context_menu_command = "Run inline commands")]
pub async fn run_inline_commands(ctx: Context<'_>, mut msg: serenity::Message) -> Result<(), Error> {
    let data = ctx.data();
    msg.guild_id = ctx.guild_id();
    match data.inline_command_log.get(&msg.id) {
        Some(_) => {
            on_message_edit(ctx.serenity_context().clone(), &msg, &data).await?;
                let builder = poise::CreateReply::default()
                .ephemeral(true)
                .content("Response updated");
            ctx.send(builder).await?;
            Ok(())
        },
        None => {
            on_message(ctx.serenity_context().clone(), &msg, &data, Some(ctx)).await
        },
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

async fn send_custom_error_message(
    ctx: poise::Context<'_, Data, Error>,
    msg: &str,
) -> Result<(), Error> {
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
    app_context: Option<Context<'_>>,
) -> Result<(), Error> {
    if msg.author.bot() {
        return Ok(());
    }
    let prompts = message_prompt_search(&msg.content);
    if let Some(context) = app_context
        && prompts.is_empty()
    {
        let builder = poise::CreateReply::default()
            .ephemeral(true)
            .content("No inline commands found in message");
        context.send(builder).await?;
    } else if !prompts.is_empty()
        && let Some(response) =
            send_inline_search_response(&ctx, msg, data, prompts, app_context).await?
    {
        let Some(guild_id) = msg.guild_id else {
            log::warn!("Failed to store message details, guild ID not found");
            return Ok(());
        };
        data.inline_command_log.insert(
            msg.id,
            BotMessageInfo {
                server_id: guild_id,
                channel_id: msg.channel_id,
                message_id: response,
                timestamp: tokio::time::Instant::now(),
            },
        );
    }
    Ok(())
}

pub async fn on_message_edit(
    ctx: serenity::Context,
    msg: &serenity::Message,
    data: &Data,
) -> Result<(), Error> {
    let Some(entry) = data.inline_command_log.get(&msg.id) else {
        return Ok(());
    };
    let bot_message_info = entry.clone();
    let message_content = &msg.content;
    if message_content.is_empty() {
        // Could not access message content, initial bot response was made via app command
        return Ok(())
    }
    let prompts = message_prompt_search(message_content);
    if !prompts.is_empty() {
        update_inline_search_response(&ctx, data, bot_message_info, prompts).await?;
        return Ok(());
    }

    // No command present in message anymore -> delete response
    let bot_message = bot_message_info.get_message(&ctx).await?;
    bot_message.delete(&ctx.http, None).await?;
    data.inline_command_log.remove(&msg.id);

    Ok(())
}

pub async fn on_message_delete(
    ctx: serenity::Context,
    channel_id: &serenity::all::GenericChannelId,
    deleted_message_id: &serenity::all::MessageId,
    data: &Data,
) -> Result<(), Error> {
    let Some(entry) = data.inline_command_log.get(deleted_message_id) else {
        return Ok(());
    };
    let message = channel_id.message(&ctx, entry.message_id).await?;
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

fn message_prompt_search(message_content: &str) -> Vec<InlineCommand> {
    let stripped_message = strip_message(message_content);
    let mut prompts = Vec::new();
    for kind in [
        InlineCommandType::Wiki,
        InlineCommandType::Faq,
        InlineCommandType::Mod,
    ] {
        prompts.push(extract_prompts(&kind, &stripped_message));
    }
    prompts.concat()
}

fn strip_message(message_content: &str) -> String {
    let mut in_code_block = false;
    let mut blockquote_depth = 0;
    let mut filtered_message = String::new();
    let events = pulldown_cmark::Parser::new(message_content);
    for event in events {
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
            pulldown_cmark::Event::End(pulldown_cmark::TagEnd::BlockQuote(None)) => {
                blockquote_depth -= 1;
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
    filtered_message
}

fn extract_prompts(kind: &InlineCommandType, filtered_message: &str) -> Vec<InlineCommand> {
    let char_vec = filtered_message.chars().collect::<Vec<char>>();
    let mut start_index: Option<usize> = None;
    let mut results = Vec::new();

    let mut start_counter = 0;
    let mut end_counter = 0;

    for (i, current_char) in char_vec.iter().enumerate() {
        if *current_char == kind.start() {
            start_counter += 1;
            if start_counter == 2 {
                start_index = Some(i + 1);
            } else if start_counter > 2 {
                start_index = None;
            }
        } else {
            start_counter = 0;
        }

        if *current_char == kind.end() {
            end_counter += 1;
        } else {
            end_counter = 0;
        }

        if let Some(s) = start_index
            && end_counter == 2
        {
            let prompt = filtered_message[s..i - 1].to_string();
            if !prompt.is_empty() {
                results.push(InlineCommand {
                    name: prompt,
                    kind: kind.clone(),
                });
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
    prompts: Vec<InlineCommand>,
    app_context: Option<Context<'_>>,
) -> Result<Option<serenity::MessageId>, Error> {
    let embeds = create_embeds_from_prompts(prompts, data, msg.guild_id).await?;
    if embeds.is_empty() {
        if let Some(context) = app_context {
            let builder = poise::CreateReply::default()
                .ephemeral(true)
                .content("No embeds generated");
            context.send(builder).await?;
        }
        Ok(None)
    } else {
        let response = if let Some(c) = app_context {
            let mut builder = poise::CreateReply::default()
                .reply(true)
                .allowed_mentions(serenity::CreateAllowedMentions::default());
            for embed in embeds {
                builder = builder.embed(embed);
            }
            c.send(builder).await?.into_message().await?
        } else {
            let builder = serenity::CreateMessage::new()
                .add_embeds(embeds)
                .reference_message(msg)
                .allowed_mentions(serenity::CreateAllowedMentions::default());
            msg.channel_id.send_message(ctx.http(), builder).await?
        };
        Ok(Some(response.id))
    }
}

async fn update_inline_search_response(
    ctx: &serenity::Context,
    data: &Data,
    bot_message_info: BotMessageInfo,
    prompts: Vec<InlineCommand>,
) -> Result<(), Error> {
    let embeds =
        create_embeds_from_prompts(prompts, data, Some(bot_message_info.server_id)).await?;
    if !embeds.is_empty() {
        let builder: serenity::EditMessage = serenity::EditMessage::new().add_embeds(embeds);
        bot_message_info
            .channel_id
            .edit_message(&ctx.http, bot_message_info.message_id, builder)
            .await?;
    }
    Ok(())
}

async fn create_embeds_from_prompts(
    prompts: Vec<InlineCommand>,
    data: &Data,
    server_id_opt: Option<serenity::GuildId>,
) -> Result<Vec<serenity::CreateEmbed<'_>>, Error> {
    let mut embeds: Vec<serenity::CreateEmbed> = Vec::new();
    for prompt in prompts {
        match prompt.kind {
            InlineCommandType::Wiki => {
                if let Some(search_result) = search_wiki_page_name(&prompt.name).await? {
                    embeds.push(wiki_commands::get_wiki_page(&search_result).await?);
                }
            }
            InlineCommandType::Mod => {
                if let Ok(embed) = commands::mod_search(prompt.name.clone(), true, data).await {
                    embeds.push(embed);
                }
            }
            InlineCommandType::Faq => {
                let cache = data.faq_cache.clone();
                let Some(server_id) = server_id_opt else {
                    continue;
                };
                let db = &data.database;
                if let Ok(embed) =
                    faq_commands::faq_core(prompt.name.clone(), cache, i64::from(server_id), db)
                        .await
                {
                    embeds.push(embed);
                }
            }
        }
    }
    Ok(embeds)
}

async fn search_wiki_page_name(name: &str) -> Result<Option<String>, Error> {
    let results = wiki_commands::opensearch_mediawiki(name).await?;
    let Some(res) = results.first() else {
        return Ok(None);
    };
    Ok(Some(res.clone()))
}

pub fn clean_inline_command_log(
    command_log: &Arc<dashmap::DashMap<serenity::MessageId, BotMessageInfo>>,
) {
    let cutoff_time = tokio::time::Instant::now() - tokio::time::Duration::from_hours(1);
    command_log.retain(|_, m| m.timestamp >= cutoff_time);
}

#[allow(clippy::cast_possible_wrap)]
pub async fn on_guild_leave(id: serenity::GuildId, db: &Pool<Sqlite>) -> Result<(), Error> {
    let server_id = id.get() as i64;
    database::clear_server_data(server_id, db).await?;
    info!("Left guild {server_id}");
    Ok(())
}
