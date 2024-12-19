use poise::serenity_prelude as serenity;
use poise::reply::CreateReply;

use crate::{custom_errors::CustomError, Context, Error,};
use crate::modding_api::lua_constants::{CHAPTERS, FUNCTIONS};

/// Link items in the Lua 5.2 manual
#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, track_edits, subcommand_required,
    subcommands("chapter", "function"), 
    install_context = "Guild|User", 
    interaction_context = "Guild|BotDm|PrivateChannel")]
pub async fn lua(
    _ctx: Context<'_>
) -> Result<(), Error> {
    Ok(())
}

/// Link chapters in the lua 5.2 manual
#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, track_edits, install_context = "Guild|User", interaction_context = "Guild|BotDm|PrivateChannel")]
pub async fn chapter (
    ctx: Context<'_>,
    #[description = "Chapter name"]
    #[autocomplete = "autocomplete_chapter"]
    #[rename = "chapter"]
    chapter_name: String,
) -> Result<(), Error> {
    if let Some(chapter) = CHAPTERS.iter().find(|ch| ch.0 == chapter_name){
        let embed = serenity::CreateEmbed::new()
            .title(chapter.0)
            .url(chapter.1)
            .author(serenity::CreateEmbedAuthor::new("Lua 5.2 Reference Manual"))
            .color(serenity::Colour::BLUE);
        let builder = CreateReply::default()
            .embed(embed);
        ctx.send(builder).await?;
    } else {
        return Err(Box::new(CustomError::new(&format!(r#"Could not find chapter "{chapter_name}" in lua manual"#))))
    }
    
    Ok(())
}

#[allow(clippy::unused_async)]
async fn autocomplete_chapter<'a>(
    _ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    CHAPTERS.iter()
        .filter(|ch| {
            let c = ch.0.to_owned();
            c.to_lowercase().contains(&partial.to_lowercase())
        })
        .map(|ch| ch.0.to_owned())
        .collect::<Vec<String>>()
}

/// Link functions in the lua 5.2 manual
#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, track_edits, install_context = "Guild|User", interaction_context = "Guild|BotDm|PrivateChannel")]
pub async fn function (
    ctx: Context<'_>,
    #[description = "function name"]
    #[autocomplete = "autocomplete_function"]
    #[rename = "function"]
    function_name: String,
) -> Result<(), Error> {
    if let Some(function) = FUNCTIONS.iter().find(|f| f.0 == function_name){
        let embed = serenity::CreateEmbed::new()
            .title(function.0)
            .url(function.1)
            .author(serenity::CreateEmbedAuthor::new("Lua 5.2 Reference Manual"))
            .color(serenity::Colour::BLUE);
        let builder = CreateReply::default()
            .embed(embed);
        ctx.send(builder).await?;
    } else {
        return Err(Box::new(CustomError::new(&format!(r#"Could not find function "{function_name}" in lua manual"#))))
    }
    Ok(())
}



#[allow(clippy::unused_async)]
async fn autocomplete_function<'a>(
    _ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    FUNCTIONS.iter()
        .filter(|f| {
            let c = f.0.to_owned();
            c.to_lowercase().contains(&partial.to_lowercase())
        })
        .map(|f| f.0.to_owned())
        .collect::<Vec<String>>()
}


