use serde::{Deserialize, Serialize};
use serenity::all::{Colour, CreateEmbed, CreateMessage};
use sqlx::{Pool, Sqlite};
use std::sync::{Arc, RwLock};

use crate::Error;
use crate::custom_errors::CustomError;
use crate::util::{escape_formatting, get_subscribed_mods, get_subscribed_authors};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiResponse {
    pub pagination: Option<Pagination>,
    pub results: Vec<Mod>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pagination {
    pub count: i32,
    pub links: Links,
    pub page: i32,
    pub page_count: i32,
    pub page_size: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Links {
    pub first: Option<String>,
    pub prev: Option<String>,
    pub next: Option<String>,
    pub last: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Mod {
    pub downloads_count: i32,
    pub latest_release: Option<Release>,
    pub name: String,
    pub owner: String,
    pub summary: String,
    pub title: String,
    pub category: Option<Category>,
    pub thumbnail: Option<String>,
    pub changelog: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Release {
    info_json: InfoJson,
    released_at: String,
    version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InfoJson {
    factorio_version: String
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    #[serde(alias = "")]
    NoCategory,
    Content,
    Overhaul,
    Tweaks,
    Utilities,
    Scenarios,
    ModPacks,
    Localizations,
    Internal,
}

impl Category {
    pub async fn to_string(&self) -> String {
        match &self {
            Self::NoCategory => "No Category".to_owned(),
            Self::Content => "Content".to_owned(),
            Self::Overhaul => "Overhaul".to_owned(),
            Self::Tweaks => "Tweaks".to_owned(),
            Self::Utilities => "Utilities".to_owned(),
            Self::Scenarios => "Scenarios".to_owned(),
            Self::ModPacks => "Mod Packs".to_owned(),
            Self::Localizations => "Localizations".to_owned(),
            Self::Internal => "Internal".to_owned(),
        }
    }
}

pub enum ModState{
    Updated,
    New,
}

pub async fn get_mods(page: i32, initializing: bool) -> Result<ApiResponse, Error> {
    
    let url: String;
    match initializing {    // Load entire database at once during initialization, use pagination when updating.
        true => {
            url = format!("https://mods.factorio.com/api/mods?page_size=max")},
        false => {
            url = format!("https://mods.factorio.com/api/mods?page_size=25&sort=updated_at&sort_order=desc&page={page}")},
    }
    let response = reqwest::get(url).await?;
    match response.status() {
        reqwest::StatusCode::OK => (),
        _ => return Err(Box::new(CustomError::new("Received HTTP status code that is not 200"))),
    };
    Ok(response.json::<ApiResponse>().await?)
}

pub async fn update_database(
        db: Pool<Sqlite>, 
        cache_http: &Arc<poise::serenity_prelude::Http>, 
        initializing: bool
    ) -> Result<(), Error> {
    let mut page = 1;
    let mut old_mod_encountered = false;
    while old_mod_encountered == false {
        let mods = get_mods(page, initializing).await?;
        page += 1;
        for result in mods.results {

            let category = match result.category.clone() {
                None => "".to_owned(),
                Some(cat) => cat.to_string().await,
            };
            let latest_release = result.latest_release.clone();
            let factorio_version = match latest_release {
                None => "".to_owned(),
                Some(ref ver) => ver.clone().info_json.factorio_version,
            };
            let version = match latest_release {
                None => "".to_owned(),
                Some(ref ver) => ver.clone().version,
            };
            let released_at = match latest_release {
                None => "".to_owned(),
                Some(ref ver) => ver.clone().released_at,
            };
            let timestamp = match chrono::DateTime::parse_from_rfc3339(&released_at) {
                Ok(datetime) => datetime.timestamp(),
                Err(_) => 0,
            };

            let state;
            let record = sqlx::query!(r#"SELECT released_at FROM mods WHERE name = ?1"#, result.name).fetch_optional(&db).await?;

            match record {
                Some(rec) => { // Mod found in database
                    if rec.released_at.unwrap_or(0) == timestamp {
                        println!("Already known mod found: {}", result.title); 
                        old_mod_encountered = true;
                        break;
                    } else {
                        state = ModState::Updated;
                        println!("Updated mod found: {}", result.title)
                    }
                },  
                None => { // Mod not found in database
                    state = ModState::New;
                    println!("New mod found: {}", result.title)
                },     
            };
            
            sqlx::query!(r#"INSERT OR REPLACE INTO mods 
                    (name, title, owner, summary, category, downloads_count, factorio_version, version, released_at)
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#, 
                    result.name,
                    result.title,
                    result.owner,
                    result.summary,
                    category,
                    result.downloads_count,
                    factorio_version,
                    version,
                    timestamp)
                    .execute(&db)
                    .await.unwrap();
            
            if !initializing {  // Only send messages when not initializing database
                let thumbnail = get_mod_thumbnail(&result.name).await?;
                let changelog = get_mod_changelog(&result.name, Some(15)).await?;
                let updated_mod = UpdatedMod{
                    name: result.name,
                    title: result.title,
                    author: result.owner,
                    version: version,
                    thumbnail: thumbnail,
                    changelog: changelog,
                    state: state
                };
                send_mod_update(updated_mod, db.clone(), &cache_http).await?;
            }
        };
        if initializing {
            break;  // Break after first loop as it retrieves all mods at once when initializing.
        }
    }
    println!("Database updated!");
    Ok(())
}

struct UpdatedMod{
    name: String,
    title: String,
    author: String,
    version: String,
    thumbnail: String,
    changelog: String,
    state: ModState,
}

struct Server {
    id: i64,
    updates_channel: Option<i64>,
    show_changelog: bool,
}

async fn send_mod_update(
        updated_mod: UpdatedMod, 
        db: Pool<Sqlite>, 
        cache_http: &Arc<poise::serenity_prelude::Http>
    ) -> Result<(), Error> {
    println!("Sending mod update message for {}", updated_mod.title);
    let server_data = sqlx::query!(r#"SELECT * FROM servers"#)
        .fetch_all(&db)
        .await?
        .into_iter()
        .map(|s| Server{
            id: s.server_id.unwrap(),
            updates_channel: s.updates_channel,
            show_changelog: s.show_changelog.unwrap_or(true)})
        .collect::<Vec<Server>>();
    for server in server_data {
        let subscribed_mods = get_subscribed_mods(&db, server.id).await?;
        let subscribed_authors = get_subscribed_authors(&db, server.id).await?;

        let updates_channel: poise::serenity_prelude::ChannelId;
        match server.updates_channel {
            Some(ch) => updates_channel = poise::serenity_prelude::ChannelId::new(ch as u64),
            None => continue,
        }

        if subscribed_mods.len() == 0 && subscribed_authors.len() == 0 {
            make_update_message(&updated_mod, updates_channel, server.show_changelog, cache_http).await?;
        }
        else if subscribed_mods.contains(&updated_mod.name) || subscribed_authors.contains(&updated_mod.author) {
            make_update_message(&updated_mod, updates_channel, server.show_changelog, cache_http).await?;
        }
    }
    Ok(())
}

async fn make_update_message(
        updated_mod: &UpdatedMod, 
        updates_channel: serenity::model::prelude::ChannelId,
        show_changelog: bool,
        cache_http: &Arc<serenity::all::Http>
    ) -> Result<(), Error> {
    let mut url = String::new();
    url.push_str("https://mods.factorio.com/mod/");
    url.push_str(&updated_mod.name);
    let color = match updated_mod.state {
        ModState::Updated => Colour::from_rgb(0x58, 0x65, 0xF2),
        ModState::New => Colour::from_rgb(0x2E, 0xCC, 0x71),
    };
    let mut title = match updated_mod.state {
        ModState::Updated => format!("Updated mod:\n{}", escape_formatting(updated_mod.title.clone()).await),
        ModState::New => format!("New mod:\n{}", escape_formatting(updated_mod.title.clone()).await),
    };
    title.truncate(265);
    let changelog: String;
    match show_changelog {
        true => changelog = updated_mod.changelog.clone(),
        false => changelog = "".to_owned(),
    };
    let author_link = format!("[{}](https://mods.factorio.com/user/{})", escape_formatting(updated_mod.author.clone()).await, &updated_mod.author);
    let embed = CreateEmbed::new()
        .title(&title)
        .url(url)
        .color(color)
        .description(changelog)
        .field("**Author**", &author_link, true)
        .field("**Version**", &updated_mod.version, true)
        .thumbnail(&updated_mod.thumbnail);
    let builder = CreateMessage::new().embed(embed);
    match updates_channel.send_message(cache_http, builder).await {
        Ok(_) => {},
        Err(e) => println!("Error sending message: {}", e),
    };
    Ok(())
}

pub async fn get_mod_thumbnail(name: &String) -> Result<String, Error> {
    let url = format!("https://mods.factorio.com/api/mods/{name}");
    let response = reqwest::get(url).await?;
    match response.status() {
        reqwest::StatusCode::OK => (),
        _ => return Err(Box::new(CustomError::new("Received HTTP status code that is not 200"))),
    };
    let mod_info = response.json::<Mod>().await?;
    let thumbnail_url = format!("https://assets-mod.factorio.com{}", mod_info.thumbnail.unwrap_or("/assets/.thumb.png".to_owned()));
    Ok(thumbnail_url)
}

pub async fn get_mod_changelog(name: &String, lines: Option<i32>) -> Result<String, Error> {
    println!("Getting mod changelog for {name}");
    let versionsplit = "-".repeat(99);
    let url = format!("https://mods.factorio.com/api/mods/{name}/full");
    let response = reqwest::get(url).await?;
    match response.status() {
        reqwest::StatusCode::OK => (),
        _ => return Err(Box::new(CustomError::new("Received HTTP status code that is not 200"))),
    };
    let mod_info = response.json::<Mod>().await?;
    match mod_info.changelog {
        Some(ch) => {
            let mut linecount = 1;
            let mut line_iter = ch.lines().skip(1);
            let mut out = String::new();
            loop {
                let l = line_iter.next().unwrap_or(&versionsplit);
                if l.contains(&versionsplit) {
                    break;
                } else if l.starts_with("    ") {
                    out.push_str(&escape_formatting(
                        l.strip_prefix("    ").unwrap().to_owned()).await
                    );
                    out.push_str("\n");
                } else if l.starts_with("  ") {
                    out.push_str("**");
                    out.push_str(&escape_formatting(
                        l.strip_prefix("  ").unwrap().to_owned()).await
                    );
                    out.push_str("**\n");
                };
                linecount += 1;
                if linecount >= lines.unwrap_or(i32::MAX) {
                    out.push_str("<Trimmed>");
                    break;
                }
            };
            out.truncate(4096);
            return Ok(out);
        },
        None => return Ok("".to_owned()),
    };
}

pub async fn get_mod_count(db: Pool<Sqlite>) -> i32 {
    let record = sqlx::query!(r#"SELECT name FROM mods"#)
        .fetch_all(&db)
        .await;
    match record {
        Ok(mods) => return mods.len() as i32,
        Err(_) => 0,
    }
}

#[derive(Debug, Clone)]
pub struct ModCacheEntry {
    pub name: String,
    pub title: String,
    pub author: String,
    pub downloads_count: i64
}

#[derive(Debug, Clone)]
pub enum SubscriptionType {
    Author(String),
    Modname(String),
}

#[derive(Debug, Clone)]
pub struct SubCacheEntry{
    pub server_id: i64,
    pub subscription: SubscriptionType 
}

pub async fn update_mod_cache(
    cache: Arc<RwLock<Vec<ModCacheEntry>>>, 
    db: Pool<Sqlite>
) -> Result<(), Error> {
    let records = sqlx::query!(r#"SELECT name, title, owner, category, downloads_count FROM mods WHERE factorio_version = ?1 ORDER BY downloads_count DESC"#, "1.1")
        .fetch_all(&db)
        .await?
        .iter()
        .map(|rec| {
            ModCacheEntry{
                name: rec.name.clone().unwrap(),
                title: rec.title.clone().unwrap_or("".to_owned()),
                author: rec.owner.clone().unwrap_or("".to_owned()),
                downloads_count: rec.downloads_count.unwrap_or(0),
            }
        })
        .collect::<Vec<ModCacheEntry>>();
    let mut w = cache.write().unwrap();
    *w = records;
    Ok(())
}

pub async fn update_sub_cache(
    cache: Arc<RwLock<Vec<SubCacheEntry>>>,
    db: Pool<Sqlite>
) -> Result<(), Error> {
    let mod_records = sqlx::query!(r#"SELECT * FROM subscribed_mods"#)
        .fetch_all(&db)
        .await?
        .iter()
        .map(|rec| {
            SubCacheEntry{
                server_id: rec.server_id.unwrap(),
                subscription: SubscriptionType::Modname(rec.mod_name.clone().unwrap())
            }
        })
        .chain(
            sqlx::query!(r#"SELECT * FROM subscribed_authors"#)
                .fetch_all(&db)
                .await?
                .iter()
                .map(|rec| {
                    SubCacheEntry{
                        server_id: rec.server_id.unwrap(),
                        subscription: SubscriptionType::Author(rec.author_name.clone().unwrap())
                    }
                })
        )
        .collect::<Vec<SubCacheEntry>>();

    let mut w = cache.write().unwrap();
    *w = mod_records;

    Ok(())
}

pub async fn update_author_cache(
    cache: Arc<RwLock<Vec<String>>>,
    db: Pool<Sqlite>
) -> Result<(), Error> {
    let mut author_records = sqlx::query!(r#"SELECT owner FROM mods"#)
        .fetch_all(&db)
        .await?
        .iter()
        .map(|rec| rec.owner.clone().unwrap())
        .collect::<Vec<String>>();
    author_records.sort_unstable();
    author_records.dedup();
    
    let mut w = cache.write().unwrap();
    *w = author_records;
    Ok(())
}