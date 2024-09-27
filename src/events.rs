use log::info;
use poise::serenity_prelude as serenity;
use regex::Regex;
use sqlx::{Pool, Sqlite};

use crate::{
    wiki_commands,
    mods::commands,
    Error,
    Data,
};

#[allow(clippy::unnecessary_unwrap)]
pub async fn on_message(ctx: serenity::Context, msg: &serenity::Message, data: &Data) -> Result<(), Error> {
    if msg.author.bot {return Ok(())};
    if let Some(wikisearch) = message_wiki_search(&msg.content).await? {
        if let Some(response) = send_wiki_message(&ctx, msg, &wikisearch).await?{
            data.inline_command_log.insert(msg.id, (msg.channel_id, response, tokio::time::Instant::now()));
        }
        return Ok(());
    }
    if let Some(modsearch) = message_mod_search(&msg.content) {
        if let Some(response) = send_mod_message(&ctx, msg, data, &modsearch).await? {
            data.inline_command_log.insert(msg.id, (msg.channel_id, response, tokio::time::Instant::now()));
        }
        return Ok(());
    }
    Ok(())
}

pub async fn on_message_edit(ctx: serenity::Context, msg: &serenity::MessageUpdateEvent, data: &Data) -> Result<(), Error> {
    if !data.inline_command_log.contains_key(&msg.id) {
        return Ok(())
    }
    let (channel_id, message_id, _) = *data.inline_command_log.get(&msg.id).unwrap();
    let Some(message_content) = &msg.content else {
        return Ok(())
    };
    if let Some(wikisearch) = message_wiki_search(message_content).await? {
        update_wiki_message(&ctx, channel_id, message_id, &wikisearch).await?;
        return Ok(())
    };

    if let Some(modsearch) = message_mod_search(message_content) {
        update_mod_message(&ctx, data, channel_id, message_id, &modsearch).await?;
        return Ok(())
    };

    Ok(())
}

#[allow(clippy::unnecessary_unwrap)]
fn message_mod_search(message_content: &str) -> Option<String> {
    let mod_regex = Regex::new(r">>(.*?)<<").unwrap();
    let neg_mod_regex = Regex::new(r"\`[\S\s]*?>>(.*?)<<[\S\s]*?\`").unwrap();
    let mod_captures = mod_regex.captures(message_content);
    let neg_mod_captures = neg_mod_regex.captures(message_content);
    if mod_captures.is_none() || neg_mod_captures.is_some() {
        None
    } else {
        Some(mod_captures.unwrap()[1].to_owned())
    }
}

async fn send_mod_message(ctx: &serenity::Context, msg: &serenity::Message, data: &Data, modname: &str) -> Result<Option<serenity::MessageId>, Error> {
    let embed = commands::mod_search(modname, true, data).await?;
    let builder: serenity::CreateMessage = serenity::CreateMessage::new().embed(embed);
    let response = msg.channel_id.send_message(&ctx, builder).await?;
    Ok(Some(response.id))
}

async fn update_mod_message(ctx: &serenity::Context, data: &Data, channel_id: serenity::ChannelId, message_id: serenity::MessageId, modname: &str) -> Result<(), Error> {
    let embed = commands::mod_search(modname, true, data).await?;
    let builder: serenity::EditMessage = serenity::EditMessage::new().embed(embed);
    channel_id.edit_message(&ctx, message_id, builder).await?;
    Ok(())
}

#[allow(clippy::unnecessary_unwrap)]
async fn message_wiki_search(message_content: &str) -> Result<Option<String>, Error> {
    let wiki_regex = Regex::new(r"\[\[(.*?)\]\]").unwrap();
    let neg_wiki_regex = Regex::new(r"\`[\S\s]*?\[\[(.*?)\]\][\S\s]*?\`").unwrap();
    if neg_wiki_regex.captures(message_content).is_some() {
        return Ok(None)
    }
    let Some(wiki_captures) = wiki_regex.captures(message_content) else {return Ok(None)};
    let wikiname = wiki_captures[1].to_owned();
    let results = wiki_commands::opensearch_mediawiki(&wikiname).await?;
    let Some(res) = results.first() else {
        return Ok(None)
    };
    Ok(Some(res.clone()))
}

async fn send_wiki_message(ctx: &serenity::Context, msg: &serenity::Message, wikiname: &str) -> Result<Option<serenity::MessageId>, Error> {
    let embed = wiki_commands::get_wiki_page(wikiname).await?;
    let builder: serenity::CreateMessage = serenity::CreateMessage::new().embed(embed);
    let response = msg.channel_id.send_message(&ctx, builder).await?;
    Ok(Some(response.id))
}

async fn update_wiki_message(ctx: &serenity::Context, channel_id: serenity::ChannelId, message_id: serenity::MessageId, wikiname: &str) -> Result<(), Error> {
    let embed = wiki_commands::get_wiki_page(wikiname).await?;
    let builder: serenity::EditMessage = serenity::EditMessage::new().embed(embed);
    channel_id.edit_message(&ctx, message_id, builder).await?;
    Ok(())
}

pub fn clean_inline_command_log(command_log: &dashmap::DashMap<serenity::MessageId, (serenity::ChannelId, serenity::MessageId, tokio::time::Instant)>) {
    let cutoff_time = tokio::time::Instant::now() - tokio::time::Duration::from_secs(3600);
    command_log.retain(|_, (_, _, t)| *t >= cutoff_time);
}

#[allow(clippy::cast_possible_wrap)]
pub async fn on_guild_leave(id: serenity::GuildId, db: Pool<Sqlite>) -> Result<(), Error> {
    let server_id = id.get() as i64;
    sqlx::query!(r#"DELETE FROM servers WHERE server_id = $1"#, server_id)
        .execute(&db)
        .await?;
    sqlx::query!(r#"DELETE FROM subscribed_mods WHERE server_id = $1"#, server_id)
        .execute(&db)
        .await?;
    sqlx::query!(r#"DELETE FROM subscribed_authors WHERE server_id = $1"#, server_id)
        .execute(&db)
        .await?;
    sqlx::query!(r#"DELETE FROM faq WHERE server_id = $1"#, server_id)
        .execute(&db)
        .await?;
    info!("Left guild {server_id}");
    Ok(())
}