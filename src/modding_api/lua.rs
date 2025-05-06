use poise::reply::CreateReply;
use poise::serenity_prelude as serenity;

use super::{
    error::ApiError,
    lua_constants::{CHAPTERS, FUNCTIONS},
};
use crate::{Context, Error, SEPARATOR};

/// Link items in the Lua 5.2 manual
#[allow(clippy::unused_async)]
#[poise::command(
    prefix_command,
    slash_command,
    track_edits,
    subcommand_required,
    subcommands("chapter", "function"),
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn lua(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Link chapters in the lua 5.2 manual
#[allow(clippy::unused_async)]
#[poise::command(
    prefix_command,
    slash_command,
    track_edits,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn chapter(
    ctx: Context<'_>,
    #[description = "Chapter name"]
    #[autocomplete = "autocomplete_chapter"]
    #[rename = "chapter"]
    #[rest]
    chapter_raw: String,
) -> Result<(), Error> {
    let chapter_name = chapter_raw
        .split_once(SEPARATOR)
        .unwrap_or((&chapter_raw, ""))
        .0
        .trim();

    if let Some(chapter) = CHAPTERS.iter().find(|ch| ch.0 == chapter_name) {
        let embed = serenity::CreateEmbed::new()
            .title(chapter.0)
            .url(chapter.1)
            .author(serenity::CreateEmbedAuthor::new("Lua 5.2 Reference Manual"))
            .color(serenity::Colour::BLUE);
        let builder = CreateReply::default().embed(embed);
        ctx.send(builder).await?;
    } else {
        return Err(ApiError::LuaChapterNotFound(chapter_name.to_string()))?;
    }

    Ok(())
}

#[allow(clippy::unused_async)]
async fn autocomplete_chapter(_ctx: Context<'_>, partial: &str) -> Vec<String> {
    CHAPTERS
        .iter()
        .filter(|ch| {
            let c = ch.0.to_owned();
            c.to_lowercase().contains(&partial.to_lowercase())
        })
        .map(|ch| ch.0.to_owned())
        .collect::<Vec<String>>()
}

/// Link functions in the lua 5.2 manual
#[allow(clippy::unused_async)]
#[poise::command(
    prefix_command,
    slash_command,
    track_edits,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn function(
    ctx: Context<'_>,
    #[description = "function name"]
    #[autocomplete = "autocomplete_function"]
    #[rename = "function"]
    #[rest]
    function_raw: String,
) -> Result<(), Error> {
    let function_name = function_raw
        .split_once(SEPARATOR)
        .unwrap_or((&function_raw, ""))
        .0
        .trim();

    if let Some(function) = FUNCTIONS.iter().find(|f| f.0 == function_name) {
        let embed = serenity::CreateEmbed::new()
            .title(function.0)
            .url(function.1)
            .author(serenity::CreateEmbedAuthor::new("Lua 5.2 Reference Manual"))
            .color(serenity::Colour::BLUE);
        let builder = CreateReply::default().embed(embed);
        ctx.send(builder).await?;
    } else {
        return Err(ApiError::LuaFunctionNotFound(function_name.to_string()))?;
    }
    Ok(())
}

#[allow(clippy::unused_async)]
async fn autocomplete_function(_ctx: Context<'_>, partial: &str) -> Vec<String> {
    FUNCTIONS
        .iter()
        .filter(|f| {
            let c = f.0.to_owned();
            c.to_lowercase().contains(&partial.to_lowercase())
        })
        .map(|f| f.0.to_owned())
        .collect::<Vec<String>>()
}
