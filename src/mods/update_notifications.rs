use serde::{Deserialize, Serialize};
use serenity::all::{Colour, CreateEmbed, CreateMessage};
use sqlx::{Pool, Sqlite};
use std::{fmt, sync::{Arc, RwLock}};
use log::{error, info};

use crate::{
    custom_errors::CustomError,
    Error,
    mods::{
        get_subscribed_authors,
        get_subscribed_mods,
    },
    formatting_tools::escape_formatting,
};

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
    #[serde(alias = "no-category")]
    Uncategorized,
    Content,
    Overhaul,
    Tweaks,
    Utilities,
    Scenarios,
    ModPacks,
    Localizations,
    Internal,
}

impl fmt::Display for Category {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Uncategorized => write!(f, "No Category"),
            Self::Content => write!(f, "Content"),
            Self::Overhaul => write!(f, "Overhaul"),
            Self::Tweaks => write!(f, "Tweaks"),
            Self::Utilities => write!(f, "Utilities"),
            Self::Scenarios => write!(f, "Scenarios"),
            Self::ModPacks => write!(f, "Mod Packs"),
            Self::Localizations => write!(f, "Localizations"),
            Self::Internal => write!(f, "Internal"),
        }
    }
}

pub enum ModState{
    Updated,
    New,
}

#[allow(clippy::module_name_repetitions)]
pub async fn get_mods(page: i32, initializing: bool) -> Result<ApiResponse, Error> {

    let url = if initializing {     // Load entire database at once during initialization, use pagination when updating.
        "https://mods.factorio.com/api/mods?page_size=max".to_string()
    } else {
        format!("https://mods.factorio.com/api/mods?page_size=25&sort=updated_at&sort_order=desc&page={page}")};
    let response = reqwest::get(url).await?;
    match response.status() {
        reqwest::StatusCode::OK => (),
        _ => return Err(Box::new(CustomError::new(&format!("Received HTTP status code {} while accessing mod portal API", response.status().as_str())))),
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
    while !old_mod_encountered {
        let mods = get_mods(page, initializing).await?;
        page += 1;
        for result in mods.results {

            let category = result.category.clone().map_or_else(String::new, |cat| format!("{cat}"));
            let latest_release = result.latest_release.clone();
            let factorio_version = latest_release.as_ref().map_or_else(String::new, |ver| ver.clone().info_json.factorio_version);
            let version = latest_release.as_ref().map_or_else(String::new, |ver| ver.clone().version);
            let released_at = latest_release.as_ref().map_or_else(String::new, |ver| ver.clone().released_at);
            let timestamp = chrono::DateTime::parse_from_rfc3339(&released_at).map_or(0, |datetime| datetime.timestamp());

            let state;
            let record = sqlx::query!(r#"SELECT released_at FROM mods WHERE name = $1"#, result.name).fetch_optional(&db).await?;

            if let Some(rec) = record { // Mod found in database
                if rec.released_at == timestamp {
                    info!("Already known mod found: {}", result.title);
                    old_mod_encountered = true;
                    break;
                }
                state = ModState::Updated;
                info!("Updated mod found: {}", result.title);
            } else { // Mod not found in database
                state = ModState::New;
                info!("New mod found: {}", result.title);
            };
            
            sqlx::query!(r#"INSERT OR REPLACE INTO mods 
                    (name, title, owner, summary, category, downloads_count, factorio_version, version, released_at)
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#, 
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
                    .await?;
            
            if !initializing {  // Only send messages when not initializing database
                let thumbnail = get_mod_thumbnail(&result.name).await?;
                let mod_info = get_mod_info(&result.name).await?;
                let changelogs = get_mod_changelog(&mod_info);
                let changelog = format_mod_changelog(&changelogs, &version, 15).unwrap_or_default();
                let updated_mod = UpdatedMod{
                    name: result.name,
                    title: result.title,
                    author: result.owner,
                    version,
                    thumbnail,
                    changelog,
                    state
                };
                send_mod_update(updated_mod, db.clone(), cache_http).await?;
            }
        };
        if initializing {
            break;  // Break after first loop as it retrieves all mods at once when initializing.
        }
    }
    info!("Database updated!");
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

#[allow(clippy::cast_sign_loss)]
async fn send_mod_update(
        updated_mod: UpdatedMod, 
        db: Pool<Sqlite>, 
        cache_http: &Arc<poise::serenity_prelude::Http>
    ) -> Result<(), Error> {
    info!("Sending mod update message for {}", updated_mod.title);
    let server_data = sqlx::query!(r#"SELECT * FROM servers"#)
        .fetch_all(&db)
        .await?
        .into_iter()
        .map(|s| { 
            Ok(Server{
                id: s.server_id,
                updates_channel: s.updates_channel,
                show_changelog: s.show_changelog.unwrap_or(true),
            })
        })
        .collect::<Vec<Result<Server, Error>>>();
    for server_res in server_data {
        let server = match server_res {
            Ok(s) => s,
            Err(e) => {
                error!{"Error sending update message: {e}"};
                continue;
            },
        };
        let subscribed_mods = get_subscribed_mods(&db, server.id).await?;
        let subscribed_authors = get_subscribed_authors(&db, server.id).await?;

        
        let updates_channel: poise::serenity_prelude::ChannelId = match server.updates_channel {
            Some(ch) => poise::serenity_prelude::ChannelId::new(ch as u64),
            None => continue,
        };

        if (subscribed_mods.is_empty() && subscribed_authors.is_empty()) || // No subscriptions
            subscribed_mods.contains(&updated_mod.name) ||      // Subscribed to mod
            subscribed_authors.contains(&updated_mod.author)    // Subscribed to author
        {
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
        ModState::Updated => format!("Updated mod:\n{}", escape_formatting(&updated_mod.title)),
        ModState::New => format!("New mod:\n{}", escape_formatting(&updated_mod.title)),
    };
    title.truncate(256);
    let changelog = if show_changelog { updated_mod.changelog.clone() } else { String::new() };
    let author_link = format!("{} ([more](https://mods.factorio.com/user/{}))", escape_formatting(&updated_mod.author), &updated_mod.author);
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
        Err(e) => error!("Error sending message: {e}"),
    };
    Ok(())
}

pub async fn get_mod_thumbnail(name: &String) -> Result<String, Error> {
    let url = format!("https://mods.factorio.com/api/mods/{name}");
    let response = reqwest::get(url).await?;
    match response.status() {
        reqwest::StatusCode::OK => (),
        _ => return Err(Box::new(CustomError::new(&format!("Received HTTP status code {} while accessing mod portal API", response.status().as_str())))),
    };
    let mod_info = response.json::<Mod>().await?;
    let thumbnail_url = format!("https://assets-mod.factorio.com{}", mod_info.thumbnail.unwrap_or_else(|| "/assets/.thumb.png".to_owned()));
    Ok(thumbnail_url)
}

#[derive(Debug, Clone, Default, PartialEq)]
struct ModChangelogEntry {
    version: String,
    date: Option<String>,
    categories: Vec<ModChangelogCategory>,
}

#[derive(Debug, Clone, Default, PartialEq)]
struct ModChangelogCategory {
    name: String,
    entries: Vec<String>,
}

async fn get_mod_info(name: &str) -> Result<Mod, Error> {
    let url = format!("https://mods.factorio.com/api/mods/{name}/full");
    let response = reqwest::get(url).await?;
    match response.status() {
        reqwest::StatusCode::OK => (),
        _ => return Err(Box::new(CustomError::new(&format!("Received HTTP status code {} while accessing mod portal API", response.status().as_str())))),
    };
    Ok(response.json::<Mod>().await?)
}

fn get_mod_changelog(mod_info: &Mod) -> Vec<ModChangelogEntry> {
    let versionsplit = "-".repeat(99);

    if mod_info.changelog.is_none() {
        return Vec::new()
    }
    let ch = mod_info.changelog.as_ref().unwrap();
    let version_entries = ch.split(&versionsplit);
    let mut out = Vec::new();
    for changelog in version_entries {
        let mut entry = ModChangelogEntry::default();
        let mut current_category = ModChangelogCategory::default();

        let lines = changelog.lines();
        for line in lines {
            if line.starts_with("Version: ") {
                if !entry.version.is_empty() {
                    entry.categories.push(current_category.clone());
                    out.push(entry.clone());
                };
                current_category = ModChangelogCategory::default();
                entry = ModChangelogEntry::default();
                line.strip_prefix("Version: ").unwrap().clone_into(&mut entry.version);
            } else if line.starts_with("Date: ") {
                entry.date = Some(line.strip_prefix("Date: ").unwrap().to_owned());
            } else if line.starts_with("    ") {
                current_category.entries.push(line.strip_prefix("    ").unwrap().to_owned());
            } else if line.starts_with("  ") {
                if !current_category.name.is_empty() {
                    entry.categories.push(current_category.clone());
                };
                current_category = ModChangelogCategory::default();
                line.strip_prefix("  ").unwrap().clone_into(&mut current_category.name);
            }
        }
        entry.categories.push(current_category.clone());
        out.push(entry);
    }
    out

}

fn format_mod_changelog(changelogs: &[ModChangelogEntry], version: &str, max_lines: usize) -> Option<String> {
    let right_changelog = changelogs.iter().find(|c| c.version == version)?;
    
    let mut lines = Vec::new();
    for category in right_changelog
        .categories.clone() 
    {
        if !category.name.is_empty() {
            lines.push(format!("**{}**", escape_formatting(&category.name)));
        }
        lines.append(&mut category.entries
            .iter()
            .map(|e| escape_formatting(e))
            .collect::<Vec<String>>()
        );
    };
    if lines.len() > max_lines {
        lines.truncate(max_lines);
        lines.push("<Trimmed>".to_owned());
    }
    Some(lines.join("\n"))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub async fn get_mod_count(db: Pool<Sqlite>) -> i32 {
    let record = sqlx::query!(r#"SELECT name FROM mods"#)
        .fetch_all(&db)
        .await;
    record.map_or(0, |mods| mods.len() as i32)
}

#[derive(Debug, Clone)]
pub struct ModCacheEntry {
    pub name: String,
    pub title: String,
    pub author: String,
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
    let records = sqlx::query!(r#"SELECT name, title, owner, downloads_count FROM mods WHERE factorio_version = $1 ORDER BY downloads_count DESC"#, "1.1")
        .fetch_all(&db)
        .await?
        .iter()
        .map(|rec| {
            ModCacheEntry{
                name: rec.name.clone(),
                title: rec.title.clone().unwrap_or_default(), // Default if mod has no name (title)
                author: rec.owner.clone(),
            }
        })
        .collect::<Vec<ModCacheEntry>>();
    match cache.write() {
        Ok(mut c) => *c = records,
        Err(e) => {
            return Err(Box::new(CustomError::new(&format!("Error acquiring cache: {e}"))));
        },
    };
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
                server_id: rec.server_id,
                subscription: SubscriptionType::Modname(rec.mod_name.clone())
            }
        })
        .chain(
            sqlx::query!(r#"SELECT * FROM subscribed_authors"#)
                .fetch_all(&db)
                .await?
                .iter()
                .filter_map(|rec| {
                    Some(SubCacheEntry{
                        server_id: rec.server_id?,
                        subscription: SubscriptionType::Author(rec.author_name.clone()?)
                    })
                })
        )
        .collect::<Vec<SubCacheEntry>>();

    match cache.write() {
        Ok(mut c) => *c = mod_records,
        Err(e) => {
            return Err(Box::new(CustomError::new(&format!("Error acquiring cache: {e}"))));
        },
    };

    Ok(())
}

pub async fn update_author_cache(
    cache: Arc<RwLock<Vec<String>>>,
    db: Pool<Sqlite>
) -> Result<(), Error> {
    let mut author_records = sqlx::query!(r#"SELECT owner FROM mods"#)
        .fetch_all(&db)
        .await?
        .into_iter()
        .map(|rec| rec.owner)
        .collect::<Vec<String>>();
    author_records.sort_unstable();
    author_records.dedup();
    
    match cache.write() {
        Ok(mut c) => *c = author_records,
        Err(e) => {
            return Err(Box::new(CustomError::new(&format!("Error acquiring cache: {e}"))));
        },
    };
    Ok(())
}

#[cfg(test)]
#[allow(unused_imports)]
mod tests{
    use super::*;
    
    #[test]
    fn try_get_changelogs() {
        let mod_info = Mod {
            downloads_count: 312_312,
            latest_release: None,
            name: String::from("Modname"),
            owner: String::from("Ownername"),
            summary: String::from("Summary String"),
            title: String::from("Title here"),
            category: None,
            thumbnail: None,
            changelog: Some(r"
Version: 1.0.1
Date: 06. 07. 2024
  Bugfixes:
    - Add partial Space Exploration support.
    - Write better tests.
  Features:
    - Add new entities.

Version: 1.0.0
  Features:
    - Initial release."
    .to_owned()),
        };
        let changelog = get_mod_changelog(&mod_info);
        // println!("{changelog:#?}");
        let expected = [
            ModChangelogEntry{ 
                version: "1.0.1".to_owned(), 
                date: Some("06. 07. 2024".to_owned()),
                categories: vec![
                    ModChangelogCategory {
                        name: "Bugfixes:".to_owned(),
                        entries: vec![
                            "- Add partial Space Exploration support.".to_owned(), 
                            "- Write better tests.".to_owned(),
                            ]
                    },
                    ModChangelogCategory {
                        name: "Features:".to_owned(),
                        entries: vec![
                            "- Add new entities.".to_owned(),
                        ]
                    }
                ]
            },
            ModChangelogEntry{ 
                version: "1.0.0".to_owned(), 
                date: None,
                categories: vec![
                    ModChangelogCategory {
                        name: "Features:".to_owned(),
                        entries: vec![
                            "- Initial release.".to_owned(),
                            ]
                    }
                ]
            },
        ];
        assert_eq!(changelog, expected);
    }

    #[test]
    fn test_format_changelog() {
        let changelog = [
            ModChangelogEntry{ 
                version: "1.0.1".to_owned(), 
                date: Some("06. 07. 2024".to_owned()),
                categories: vec![
                    ModChangelogCategory {
                        name: "Bugfixes:".to_owned(),
                        entries: vec![
                            "- Add partial Space Exploration support.".to_owned(), 
                            "- Write better tests.".to_owned(),
                            ]
                    },
                    ModChangelogCategory {
                        name: "Features:".to_owned(),
                        entries: vec![
                            "- Add new entities.".to_owned(),
                        ]
                    }
                ]
            },
            ModChangelogEntry{ 
                version: "1.0.0".to_owned(), 
                date: None,
                categories: vec![
                    ModChangelogCategory {
                        name: "Features:".to_owned(),
                        entries: vec![
                            "- Initial release.".to_owned(),
                            ]
                    }
                ]
            },
        ];
        let formatted_changelog = format_mod_changelog(&changelog, "1.0.1", 15);
        let expected_output = Some(
r"**Bugfixes:**
- Add partial Space Exploration support.
- Write better tests.
**Features:**
- Add new entities.".to_owned());
        assert_eq!(formatted_changelog, expected_output);
    }
}