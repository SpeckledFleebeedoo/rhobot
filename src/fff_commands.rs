use poise::CreateReply;
use poise::serenity_prelude::{Colour, CreateEmbed};
use scraper::{Html, Selector};
use std::{error, fmt};

use crate::{Context, Error, formatting_tools::DiscordFormat};

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

#[derive(Debug)]
pub enum FFFError {
    SendMessageFailed(serenity::Error),
    ReqwestError(reqwest::Error),
    PageNotFound(i32),
    BadStatusCode(String),
    HeadNotFound,
    HeadInvalid,
    TitleNotFound,
    TitleInvalid,
    ThumbnailNotFound,
    ThumbnailInvalid,
    BodyNotFound,
    BodyInvalid,
}

impl fmt::Display for FFFError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ReqwestError(error) => f.write_str(&format!("Error retrieving FFF: {error}.")),
            Self::PageNotFound(number) => f.write_str(&format!("Page for FFF {number} not found.")),
            Self::BadStatusCode(status) => f.write_str(&format!(
                "Received HTTP status code {status} while accessing FFF website."
            )),
            Self::HeadNotFound => f.write_str("Failed to read FFF page: html `head` not found"),
            Self::HeadInvalid => f.write_str("Failed to read FFF page: invalid html `head`"),
            Self::TitleNotFound => f.write_str("Failed to read FFF page: could not find title"),
            Self::TitleInvalid => f.write_str("Failed to read FFF page: failed to read title"),
            Self::ThumbnailNotFound => {
                f.write_str("Failed to read FFF page: could not find thumbnail")
            }
            Self::ThumbnailInvalid => {
                f.write_str("Failed to read FFF page: failed to parse thumbnail url")
            }
            Self::BodyNotFound => f.write_str("Failed to read FFF page: could not find body text"),
            Self::BodyInvalid => f.write_str("Failed to read FFF page: failed to parse body text"),
            Self::SendMessageFailed(error) => {
                f.write_str(&format!("Failed to send message: {error}"))
            }
        }
    }
}

impl error::Error for FFFError {}

impl From<reqwest::Error> for FFFError {
    fn from(value: reqwest::Error) -> Self {
        Self::ReqwestError(value)
    }
}

impl From<serenity::Error> for FFFError {
    fn from(value: serenity::Error) -> Self {
        Self::SendMessageFailed(value)
    }
}

async fn get_fff_data(number: i32) -> Result<FFFData, FFFError> {
    let url = format!("https://www.factorio.com/blog/post/fff-{number}");
    let response = reqwest::get(&url).await.map_err(FFFError::from)?;
    match response.status() {
        reqwest::StatusCode::OK => (),
        reqwest::StatusCode::NOT_FOUND => return Err(FFFError::PageNotFound(number)),
        _ => return Err(FFFError::BadStatusCode(response.status().to_string())),
    }
    let mut fff = FFFData::new(url);
    let text = response.text().await?;
    let document = Html::parse_document(&text);

    let head_selector = Selector::parse("head").map_err(|_| FFFError::HeadNotFound)?;
    let head = document
        .select(&head_selector)
        .next()
        .ok_or_else(|| FFFError::HeadInvalid)?;

    let title_selector =
        Selector::parse(r#"meta[property="og:title"]"#).map_err(|_| FFFError::TitleNotFound)?;
    let title_element = head
        .select(&title_selector)
        .next()
        .ok_or_else(|| FFFError::TitleInvalid)?;
    fff.title = title_element.value().attr("content").map(|f| {
        f.trim_end_matches("| Factorio")
            .to_owned()
            .truncate_for_embed(256)
    });

    let image_selector =
        Selector::parse(r#"meta[property="og:image"#).map_err(|_| FFFError::ThumbnailNotFound)?;
    let image_element = head
        .select(&image_selector)
        .next()
        .ok_or_else(|| FFFError::ThumbnailInvalid)?;
    fff.image = image_element
        .value()
        .attr("content")
        .map(std::borrow::ToOwned::to_owned);

    let description_selector =
        Selector::parse(r#"meta[property="og:description"#).map_err(|_| FFFError::BodyNotFound)?;
    let description_element = head
        .select(&description_selector)
        .next()
        .ok_or_else(|| FFFError::BodyInvalid)?;
    fff.description = description_element
        .value()
        .attr("content")
        .map(|f| f.to_owned().truncate_for_embed(1000));
    Ok(fff)
}

pub fn fff() -> poise::Command<crate::Data, Error> {
    poise::Command {
        slash_action: fff_slash().slash_action,
        parameters: fff_slash().parameters,
        install_context: fff_slash().install_context,
        interaction_context: fff_slash().interaction_context,
        ..fff_prefix()
    }
}

/// Link an FFF
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn fff_slash(
    ctx: Context<'_>,
    #[description = "Number of the FFF"] number: i32,
) -> Result<(), Error> {
    fff_core(ctx, number).await?;
    Ok(())
}

/// Link an FFF
#[poise::command(prefix_command, hide_in_help, track_edits, rename = "fff")]
pub async fn fff_prefix(
    ctx: Context<'_>,
    #[description = "Number of the FFF"] number: Option<i32>,
    #[rest] _rest: Option<String>,
) -> Result<(), Error> {
    if let Some(n) = number {
        fff_core(ctx, n).await?;
    } else {
        fff_default(ctx).await?;
    }
    Ok(())
}

async fn fff_core(ctx: Context<'_>, number: i32) -> Result<(), FFFError> {
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

async fn fff_default(ctx: Context<'_>) -> Result<(), FFFError> {
    let embed = CreateEmbed::new()
        .title("Factorio Friday Facts")
        .url("https://www.factorio.com/blog")
        .thumbnail("https://factorio.com/static/img/factorio-wheel.png")
        .color(Colour::ORANGE);
    let builder = CreateReply::default().embed(embed);
    ctx.send(builder).await?;
    Ok(())
}
