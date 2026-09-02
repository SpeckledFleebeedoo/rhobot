use poise::reply::CreateReply;
use poise::serenity_prelude as serenity;

use super::{
    error::ApiError,
    lua_constants::{CHAPTERS, FUNCTIONS},
};
use crate::{Context, Error};

/// Link items in the Lua 5.2 manual
#[allow(clippy::unused_async)]
#[poise::command(
    slash_command,
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
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn chapter(
    ctx: Context<'_>,
    #[description = "Chapter name"]
    #[autocomplete = "autocomplete_chapter"]
    #[rename = "chapter"]
    chapter_name: String,
) -> Result<(), Error> {
    if let Some(chapter) = CHAPTERS.iter().find(|ch| ch.0 == chapter_name) {
        let embed = serenity::CreateEmbed::new()
            .title(chapter.0)
            .url(chapter.1)
            .author(serenity::CreateEmbedAuthor::new("Lua 5.2 Reference Manual"))
            .color(serenity::Colour::BLUE);
        let builder = CreateReply::default()
            .embed(embed)
            .reply(true)
            .allowed_mentions(serenity::CreateAllowedMentions::default());
        ctx.send(builder).await?;
    } else {
        return Err(ApiError::LuaChapterNotFound(chapter_name))?;
    }

    Ok(())
}

#[allow(clippy::unused_async)]
async fn autocomplete_chapter<'a>(_ctx: Context<'a>, partial: &'a str) -> serenity::CreateAutocompleteResponse<'a> {
    let choices = CHAPTERS
        .iter()
        .filter(|ch| {
            let c = ch.0.to_owned();
            c.to_lowercase().contains(&partial.to_lowercase())
        })
        .map(|ch| serenity::AutocompleteChoice::from(ch.0.to_owned()))
        .take(25)
        .collect::<Vec<serenity::AutocompleteChoice>>();
    serenity::CreateAutocompleteResponse::new().set_choices(choices)
}

/// Link functions in the lua 5.2 manual
#[allow(clippy::unused_async)]
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn function(
    ctx: Context<'_>,
    #[description = "function name"]
    #[autocomplete = "autocomplete_function"]
    #[rename = "function"]
    function_name: String,
) -> Result<(), Error> {
    if let Some(function) = FUNCTIONS.iter().find(|f| f.0 == function_name) {
        let embed = serenity::CreateEmbed::new()
            .title(function.0)
            .url(function.1)
            .author(serenity::CreateEmbedAuthor::new("Lua 5.2 Reference Manual"))
            .color(serenity::Colour::BLUE);
        let builder = CreateReply::default()
            .embed(embed)
            .reply(true)
            .allowed_mentions(serenity::CreateAllowedMentions::default());
        ctx.send(builder).await?;
    } else {
        return Err(ApiError::LuaFunctionNotFound(function_name))?;
    }
    Ok(())
}

#[allow(clippy::unused_async)]
async fn autocomplete_function<'a>(_ctx: Context<'a>, partial: &'a str) -> serenity::CreateAutocompleteResponse<'a> {
    let choices = FUNCTIONS
        .iter()
        .filter(|f| {
            let c = f.0.to_owned();
            c.to_lowercase().contains(&partial.to_lowercase())
        })
        .map(|f| serenity::AutocompleteChoice::from(f.0.to_owned()))
        .take(25)
        .collect::<Vec<serenity::AutocompleteChoice>>();
    
    serenity::CreateAutocompleteResponse::new().set_choices(choices)
}
