use poise::serenity_prelude::{CreateEmbed, Colour};
use poise::CreateReply;
use scraper::{Html, Selector};

use crate::{
    Context, 
    custom_errors::CustomError, 
    Error, 
    formatting_tools::DiscordFormat
};

#[derive(Debug)]
struct FFFData {
    url: String,
    title: Option<String>,
    image: Option<String>,
    description: Option<String>,
}

impl FFFData {
    pub const fn new(url: String) -> Self {
        Self {
            url,
            title: None,
            image: None,
            description: None,
        }
    }
}

async fn get_fff_data(number: i32) -> Result<FFFData, Error> {
    let url = format!("https://www.factorio.com/blog/post/fff-{number}");
    let response = reqwest::get(&url).await?;
    match response.status() {
        reqwest::StatusCode::OK => (),
        reqwest::StatusCode::NOT_FOUND => {return Err(Box::new(CustomError::new("Page does not exist")))},
        _ => return Err(Box::new(CustomError::new(&format!("Received HTTP status code {} while accessing FFF website", response.status().as_str())))),
    };
    let mut fff = FFFData::new(url);
    let text = response.text().await?;
    let document = Html::parse_document(&text);

    let Ok(head_selector) = Selector::parse("head") 
        else {return Err(Box::new(CustomError::new("Failed to read FFF page: html `head` not found")))};
    let Some(head) = document.select(&head_selector).next()
        else {return Err(Box::new(CustomError::new("Failed to read FFF page: invalid html `head`")))};

    let Ok(title_selector) = Selector::parse(r#"meta[property="og:title"]"#)
        else {return Err(Box::new(CustomError::new("Failed to read FFF page: could not find title")))};
    let Some(title_element) = head.select(&title_selector).next()
        else {return Err(Box::new(CustomError::new("Failed to read FFF page: failed to read title")))};
    fff.title = title_element.value().attr("content").map(|f| {
        f.trim_end_matches("| Factorio").to_owned().truncate_for_embed(256)
    });

    let Ok(image_selector) = Selector::parse(r#"meta[property="og:image"#)
        else {return Err(Box::new(CustomError::new("Failed to read FFF page: could not find thumbnail")))};
    let Some(image_element) = head.select(&image_selector).next()
        else {return Err(Box::new(CustomError::new("Failed to read FFF page: failed to parse thumbnail url")))};
    fff.image = image_element.value().attr("content").map(std::borrow::ToOwned::to_owned);

    let Ok(description_selector) = Selector::parse(r#"meta[property="og:description"#)
        else {return Err(Box::new(CustomError::new("Failed to read FFF page: could not find body text")))};
    let Some(description_element) = head.select(&description_selector).next()
        else {return Err(Box::new(CustomError::new("Failed to read FFF page: failed to parse body text")))};
    fff.description = description_element.value().attr("content").map(|f| {
        f.to_owned().truncate_for_embed(1000)
    });
    Ok(fff)
}

pub fn fff() -> poise::Command<crate::Data, Box<dyn std::error::Error + Send + Sync>> {
    poise::Command {
        slash_action: fff_slash().slash_action,
        parameters: fff_slash().parameters,
        install_context: fff_slash().install_context,
        interaction_context: fff_slash().interaction_context,
        ..fff_prefix()
    }
}

/// Link an FFF
#[poise::command(slash_command, install_context = "Guild|User", interaction_context = "Guild|BotDm|PrivateChannel")]
pub async fn fff_slash(
    ctx: Context<'_>,
    #[description = "Number of the FFF"]
    number: i32,
) -> Result<(), Error> {
    fff_core(ctx, number).await?;
    Ok(())
}

/// Link an FFF
#[poise::command(prefix_command, hide_in_help, track_edits, rename = "fff")]
pub async fn fff_prefix(
    ctx: Context<'_>,
    #[description = "Number of the FFF"]
    number: Option<i32>,
    #[rest]
    _rest: Option<String>,
) -> Result<(), Error> {
    if let Some(n) = number {
        fff_core(ctx, n).await?;
    } else {
        fff_default(ctx).await?;
    };
    Ok(())
}

async fn fff_core(
    ctx: Context<'_>,
    number: i32,
) -> Result<(), Error> {
    let fff_data = get_fff_data(number).await?;
    let embed = CreateEmbed::new()
        .title(fff_data.title.unwrap_or_default())
        .url(fff_data.url)
        .description(fff_data.description.unwrap_or_default())
        .thumbnail(fff_data.image.unwrap_or_default())
        .color(Colour::ORANGE);
    let builder = CreateReply::default().embed(embed);
    ctx.send(builder).await?;
    Ok(())
}

async fn fff_default(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let embed = CreateEmbed::new()
        .title("Factorio Friday Facts")
        .url("https://www.factorio.com/blog")
        .thumbnail("https://factorio.com/static/img/factorio-wheel.png")
        .color(Colour::ORANGE);
    let builder = CreateReply::default().embed(embed);
    ctx.send(builder).await?;
    Ok(())
}