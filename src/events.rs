use log::{error, info};
use poise::CreateReply;
use poise::serenity_prelude as serenity;
use sqlx::{Pool, Sqlite};

use crate::{Context, Data, Error, database, mods::commands, wiki_commands};

pub async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {error}"),
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
    let builder = CreateReply::default().embed(embed);
    ctx.send(builder).await?;
    Ok(())
}

#[allow(clippy::unnecessary_unwrap)]
pub async fn on_message(
    ctx: serenity::Context,
    msg: &serenity::Message,
    data: &Data,
) -> Result<(), Error> {
    if msg.author.bot {
        return Ok(());
    }
    let wikisearch = message_prompt_search(&msg.content, '[', ']');
    let modsearch = message_prompt_search(&msg.content, '>', '<');
    if !modsearch.is_empty() || !wikisearch.is_empty() {
        if let Some(response) = send_inline_search_response(&ctx, msg, data, modsearch, wikisearch).await? {
            data.inline_command_log.insert(
                msg.id,
                (msg.channel_id, response, tokio::time::Instant::now()),
            );
        }
        return Ok(())
    }
    Ok(())
}

pub async fn on_message_edit(
    ctx: serenity::Context,
    msg: &serenity::MessageUpdateEvent,
    data: &Data,
) -> Result<(), Error> {
    if !data.inline_command_log.contains_key(&msg.id) {
        return Ok(());
    }
    let (channel_id, message_id, _) = *data.inline_command_log.get(&msg.id).unwrap();
    let Some(message_content) = &msg.content else {
        return Ok(());
    };
    let wikisearch = message_prompt_search(message_content, '[', ']');
    let modsearch = message_prompt_search(message_content, '>', '<');
    if !modsearch.is_empty() || !wikisearch.is_empty() {
        update_inline_search_response(&ctx, data, channel_id, message_id, modsearch, wikisearch).await?;
        return Ok(())
    }

    // No command present in message anymore -> delete response
    let message = channel_id.message(&ctx, message_id).await?;
    message.delete(&ctx).await?;
    data.inline_command_log.remove(&msg.id);

    Ok(())
}

pub async fn on_message_delete(
    ctx: serenity::Context,
    channel_id: &serenity::all::ChannelId,
    deleted_message_id: &serenity::all::MessageId,
    data: &Data,
) -> Result<(), Error> {
    if !data.inline_command_log.contains_key(deleted_message_id) {
        return Ok(());
    }
    let (_, message_id, _) = *data.inline_command_log.get(deleted_message_id).unwrap();
    let message = channel_id.message(&ctx, message_id).await?;
    message.delete(&ctx).await?;
    data.inline_command_log.remove(deleted_message_id);

    Ok(())
}

fn message_prompt_search(message_content: &str, opening_char: char, closing_char: char) -> Vec<String> {
    let mut in_code_block = false;
    let mut blockquote_depth = 0;
    let mut filtered_message = String::new();
    for event in pulldown_cmark::Parser::new(message_content) {
        match event {
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::CodeBlock(_)) => {
                in_code_block = true;
            },
            pulldown_cmark::Event::End(pulldown_cmark::TagEnd::CodeBlock) => {
                in_code_block = false;
            },
            pulldown_cmark::Event::Start(pulldown_cmark::Tag::BlockQuote(None)) => {
                filtered_message.push('>');
                blockquote_depth += 1;
            },
            pulldown_cmark::Event::SoftBreak => {
                for _ in 0..blockquote_depth {
                    filtered_message.push('>');
                }
            },
            pulldown_cmark::Event::Text(pulldown_cmark::CowStr::Borrowed(text)) if !in_code_block => {
                filtered_message.push_str(text);
            },
            pulldown_cmark::Event::End(pulldown_cmark::TagEnd::Paragraph) => {
                filtered_message.push('\n');
            },
            _ => ()
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
                start_index = Some(i+1);
            } else if start_counter > 2 {
                start_index = None;
            }
            end_counter = 0;
        } else if current_char == closing_char {
            end_counter += 1;
            start_counter = 0;
        }

        if let Some(s) = start_index && end_counter == 2 {
            let modname = filtered_message[s..i-1].to_string();
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
    for modname in modnames {
        if let Ok(embed) = commands::mod_search(&modname, true, data).await {
            embeds.push(embed);
        }
    }
    for wikiname in wikinames {
        if let Some(search_result) = search_wiki_page_name(&wikiname).await? {
            embeds.push(wiki_commands::get_wiki_page(&search_result).await?);
        }
    }
    if embeds.is_empty() {
        Ok(None)
    } else {
        let builder: serenity::CreateMessage = serenity::CreateMessage::new().add_embeds(embeds);
        let response = msg.channel_id.send_message(&ctx, builder).await?;
        Ok(Some(response.id))
    }
}

async fn update_inline_search_response(
    ctx: &serenity::Context,
    data: &Data,
    channel_id: serenity::ChannelId,
    message_id: serenity::MessageId,
    modnames: Vec<String>,
    wikinames: Vec<String>,
) -> Result<(), Error> {
    let mut embeds: Vec<serenity::CreateEmbed> = Vec::new();
    for modname in modnames {
        if let Ok(embed) = commands::mod_search(&modname, true, data).await {
            embeds.push(embed);
        }
    }
    for wikiname in wikinames {
        if let Some(search_result) = search_wiki_page_name(&wikiname).await? {
            embeds.push(wiki_commands::get_wiki_page(&search_result).await?);
        }
    }
    if !embeds.is_empty() {
        let builder: serenity::EditMessage = serenity::EditMessage::new().add_embeds(embeds);
        channel_id.edit_message(&ctx, message_id, builder).await?;
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
    command_log: &dashmap::DashMap<
        serenity::MessageId,
        (
            serenity::ChannelId,
            serenity::MessageId,
            tokio::time::Instant,
        ),
    >,
) {
    let cutoff_time = tokio::time::Instant::now() - tokio::time::Duration::from_secs(3600);
    command_log.retain(|_, (_, _, t)| *t >= cutoff_time);
}

#[allow(clippy::cast_possible_wrap)]
pub async fn on_guild_leave(id: serenity::GuildId, db: &Pool<Sqlite>) -> Result<(), Error> {
    let server_id = id.get() as i64;
    database::clear_server_data(server_id, db).await?;
    info!("Left guild {server_id}");
    Ok(())
}
