use std::sync::Arc;
use poise::serenity_prelude::{CreateEmbed, Colour, ChannelId, EditChannel, Http};
use poise::CreateReply;
use scraper::{Html, Selector};
use chrono::{DateTime, Datelike, TimeZone, Timelike};
use chrono_tz::{Europe::Prague, Tz};
use log::{error, info};
use crate::{Context, Error, custom_errors::CustomError};

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
        _ => return Err(Box::new(CustomError::new("Received HTTP status code that is not 200"))),
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
        let mut title = f.trim_end_matches("| Factorio").to_owned();
        title.truncate(256);
        title
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
        let mut desc = f.to_owned();
        desc.truncate(1000);
        desc
    });
    Ok(fff)
}

/// Link an FFF
#[poise::command(prefix_command, slash_command, track_edits)]
pub async fn fff(
    ctx: Context<'_>,
    #[description = "Number of the FFF"]
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

#[allow(clippy::unreadable_literal)]
pub async fn update_fff_channel_description(cache_http: Arc<Http>) {
    // discord.gg/factorio #friday-facts channel
    let fff_channel = ChannelId::from(603392474458882065);
    let Some(next_fff_time) = next_friday_1pm() else {
        error!("Error while updating FFF timestamp");
        return ;
    };
    let topic = format!("FFF <t:{next_fff_time}:R> - \nIn Friday Facts We Trust: https://www.factorio.com/blog/");
    let builder = EditChannel::new().topic(topic);
    match fff_channel.edit(&cache_http, builder).await {
        Ok(_) => info!("Updated FFF timestamp"),
        Err(error) => error!("Error while updating FFF timestamp: {error}"),
    };
}

// Find unix timestamp of next friday 1pm CET/CEST
#[allow(clippy::cast_lossless)]
fn next_friday_1pm() -> Option<i64> {
    let prague_now: DateTime<Tz> = Prague.from_utc_datetime(&chrono::Utc::now().naive_utc());
    let weekday = prague_now.weekday();
    let mut days_to_friday = (chrono::Weekday::Fri.number_from_sunday() - weekday.number_from_sunday() + 7) % 7;
    if days_to_friday == 0 && prague_now.hour() >= 13 {
        days_to_friday = 7;
    };

    let next_friday = prague_now.date_naive().and_hms_opt(13, 0, 0)? + chrono::Duration::days(days_to_friday as i64);
    let next_friday_prague = Prague.from_local_datetime(&next_friday).unwrap();

    Some(next_friday_prague.timestamp())
}