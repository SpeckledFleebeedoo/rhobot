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
    let wiki_regex = Regex::new(r"\[\[(.*?)\]\]").unwrap();
    let neg_wiki_regex = Regex::new(r"\`[\S\s]*?\[\[(.*?)\]\][\S\s]*?\`").unwrap();
    let wiki_captures = wiki_regex.captures(&msg.content);
    let neg_wiki_captures = neg_wiki_regex.captures(&msg.content);
    let wiki_search = if wiki_captures.is_none() || neg_wiki_captures.is_some() {
        None
    } else {
        Some(wiki_captures.unwrap()[1].to_owned())
    };
    
    let mod_regex = Regex::new(r">>(.*?)<<").unwrap();
    let neg_mod_regex = Regex::new(r"\`[\S\s]*?>>(.*?)<<[\S\s]*?\`").unwrap();
    let mod_captures = mod_regex.captures(&msg.content);
    let neg_mod_captures = neg_mod_regex.captures(&msg.content);
    let mod_search = if mod_captures.is_none() || neg_mod_captures.is_some() {
        None
    } else {
        Some(mod_captures.unwrap()[1].to_owned())
    };

    if let Some(result_str) = wiki_search {
        let results = wiki_commands::opensearch_mediawiki(&result_str).await?;
        let Some(res) = results.first() else {
            return Ok(())
        };
    
        let embed = wiki_commands::get_wiki_page(res).await?;
        let http = ctx.http.clone();
        let builder: serenity::CreateMessage = serenity::CreateMessage::new().embed(embed);
        msg.channel_id.send_message(http, builder).await?;
    };
    if let Some(result_str) = mod_search {
        let embed = commands::mod_search(&result_str, true, data).await?;
        let http = ctx.http.clone();
        let builder: serenity::CreateMessage = serenity::CreateMessage::new().embed(embed);
        msg.channel_id.send_message(http, builder).await?;
    }
    Ok(())
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