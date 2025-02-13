use sqlx::{Pool, Sqlite};
use std::collections::HashMap;

use crate::Error;
use crate::faq_commands::{BasicFaqEntry, FaqCacheEntry};
use crate::mods::update_notifications::{ModCacheEntry, SubCacheEntry, SubscriptionType};


pub async fn clear_server_data(server_id: i64, db: &Pool<Sqlite>) -> Result<(), Error> {
    sqlx::query!(r#"DELETE FROM servers WHERE server_id = $1"#, server_id)
        .execute(db)
        .await?;
    sqlx::query!(r#"DELETE FROM subscribed_mods WHERE server_id = $1"#, server_id)
        .execute(db)
        .await?;
    sqlx::query!(r#"DELETE FROM subscribed_authors WHERE server_id = $1"#, server_id)
        .execute(db)
        .await?;
    sqlx::query!(r#"DELETE FROM faq WHERE server_id = $1"#, server_id)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn get_server_faqs(server_id: i64, db: &Pool<Sqlite>) -> Result<HashMap<String, Vec<String>>, Error> {
    let db_entries = sqlx::query!(r#"SELECT title, link FROM faq WHERE server_id = $1"#, server_id)
        .fetch_all(db)
        .await?;
    let mut faq_map: HashMap<String, Vec<String>> = HashMap::new();
    let base_faqs: Vec<String> = db_entries.iter().filter(|f| f.link.is_none()).map(|f| f.title.clone()).collect();
    let link_faqs: Vec<(String, String)> = db_entries.iter()
        .filter_map(|f| f.link
            .clone()
            .map(|l| (f.title.clone(), l))
        )
        .collect();
    
    for entry in &base_faqs {
        faq_map.insert(entry.clone(), Vec::new());
    }
    for (faq, link) in link_faqs {
        if let Some(map) = faq_map.get_mut(&link) {
            map.push(faq);
        }
    }
    Ok(faq_map)
}

pub async fn get_faq_titles(db: &Pool<Sqlite>,) -> Result<Vec<FaqCacheEntry>, Error> {
    let records = sqlx::query_as!(FaqCacheEntry, r#"SELECT server_id, title FROM faq"#)
        .fetch_all(db)
        .await?;
    Ok(records)
}

pub async fn get_server_faq_dump(db: &Pool<Sqlite>, server_id: i64) -> Result<Vec<BasicFaqEntry>, Error> {
    let server_faqs = sqlx::query_as!(BasicFaqEntry, r#"SELECT title, contents, image, link FROM faq WHERE server_id = $1"#, server_id)
        .fetch_all(db)
        .await?;
    Ok(server_faqs)
}

pub async fn delete_faq_entry(db: &Pool<Sqlite>, server_id: i64, name: &str) -> Result<u64, Error> {
    Ok(sqlx::query!(r#"DELETE FROM faq WHERE server_id = $1 AND title = $2"#, server_id, name)
        .execute(db)
        .await?
        .rows_affected())
}

pub async fn clear_server_faq(db: &Pool<Sqlite>, server_id: i64) -> Result<(), Error> {
    sqlx::query!(r#"DELETE FROM faq WHERE server_id = $1"#, server_id)
        .execute(db)
        .await?;
    Ok(())
}

pub struct DBFaqEntry<'a> {
    pub server_id: i64, 
    pub name: &'a str, 
    pub content: Option<&'a str>, 
    pub attachment_url: Option<&'a str>, 
    pub timestamp: i64, 
    pub author_id: i64,
    pub link: Option<&'a str>,
}

pub async fn add_faq_entry<'a>(
    db: &Pool<Sqlite>, 
    faq_entry: DBFaqEntry<'a>,
) -> Result<(), Error> {
    sqlx::query!(
        r#"INSERT INTO faq (server_id, title, contents, image, edit_time, author, link)
        VALUES (?, ?, ?, ?, ?, ?, ?)"#,
        faq_entry.server_id,
        faq_entry.name,
        faq_entry.content,
        faq_entry.attachment_url,
        faq_entry.timestamp,
        faq_entry.author_id,
        faq_entry.link
    )
    .execute(db)
    .await?;
    Ok(())
}

pub async fn find_faq_entry_opt(db: &Pool<Sqlite>, server_id: i64, name: &str) -> Result<Option<BasicFaqEntry>, Error> {
    Ok(sqlx::query_as!(BasicFaqEntry, 
        r#"SELECT title, contents, image, link FROM faq WHERE server_id = $1 AND title = $2"#, server_id, name)
        .fetch_optional(db)
        .await?)
}

pub async fn get_modrole(db: &Pool<Sqlite>, server_id: i64) -> Result<Option<i64>, Error> {
    let role = sqlx::query!(r#"SELECT modrole FROM servers WHERE server_id = $1"#, server_id)
        .fetch_one(db)
        .await?
        .modrole;
    Ok(role)
}

pub struct DBServerInfo {
    pub server_id: i64,
    pub updates_channel: Option<i64>,
    pub modrole: Option<i64>,
    pub show_changelog: Option<bool>,
}

pub async fn get_server_info(db: &Pool<Sqlite>, server_id: i64) -> Result<Option<DBServerInfo>, Error> {
    let serverdata = sqlx::query_as!(DBServerInfo, r#"SELECT * FROM servers WHERE server_id = $1"#, server_id)
        .fetch_optional(db)
        .await?;
    Ok(serverdata)
}

pub async fn get_all_servers(db: &Pool<Sqlite>) -> Result<Vec<DBServerInfo>, Error> {
    let server_data = sqlx::query_as!(DBServerInfo, r#"SELECT * FROM servers"#)
        .fetch_all(db)
        .await?;
    Ok(server_data)
}

pub async fn get_subscribed_mods(db: &Pool<Sqlite>, server_id: i64) -> Result<Vec<String>, Error> {
    let subscribed_mods = sqlx::query!(r#"SELECT mod_name FROM subscribed_mods WHERE server_id = $1"#, server_id)
        .fetch_all(db)
        .await?
        .into_iter()
        .map(|m| m.mod_name)
        .collect::<Vec<String>>();
    Ok(subscribed_mods)
}

pub async fn get_subscribed_authors(db: &Pool<Sqlite>, server_id: i64) -> Result<Vec<String>, Error> {
    let subscribed_authors = sqlx::query!(r#"SELECT author_name FROM subscribed_authors WHERE server_id = $1"#, server_id)
        .fetch_all(db)
        .await?
        .into_iter()
        .filter_map(|m| m.author_name)
        .collect::<Vec<String>>();
    Ok(subscribed_authors)
}

pub async fn store_updates_channel(db: &Pool<Sqlite>, server_id: i64, channel_id: i64) -> Result<(), Error> {
    if (sqlx::query!(r#"SELECT * FROM servers WHERE server_id = $1"#, server_id)
        .fetch_optional(db)
        .await?).is_some() {
        // Update server data if it does exist
        sqlx::query!(r#"UPDATE servers SET updates_channel = $1 WHERE server_id = $2"#,
        channel_id, server_id)
            .execute(db)
            .await?;
    } else {
        // Add server and set setting if it does not exist
        sqlx::query!(r#"INSERT INTO servers (server_id, updates_channel) VALUES ($1, $2)"#,
        server_id, channel_id)
            .execute(db)
            .await?;
    };
    Ok(())
}

pub async fn store_modrole(db: &Pool<Sqlite>, server_id: i64, role_id: i64) -> Result<(), Error> {
    if (sqlx::query!(r#"SELECT * FROM servers WHERE server_id = $1"#, server_id)
        .fetch_optional(db)
        .await?).is_some() {
        // Update server data if it does exist
        sqlx::query!(r#"UPDATE servers SET modrole = $1 WHERE server_id = $2"#,
        role_id, server_id)
            .execute(db)
            .await?;
    } else {
        // Add server and set setting if it does not exist
        sqlx::query!(r#"INSERT INTO servers (server_id, modrole) VALUES ($1, $2)"#,
        server_id, role_id)
            .execute(db)
            .await?;
    };
    Ok(())
}

pub async fn store_changelog_setting(db: &Pool<Sqlite>, server_id: i64, show_changelogs: bool) -> Result<(), Error> {
    match sqlx::query!(r#"SELECT server_id FROM servers WHERE server_id = $1"#, server_id)
            .fetch_optional(db)
            .await? {
        Some(_) => {
            // Update server data if it does exist
            sqlx::query!(r#"UPDATE servers SET show_changelog = $1 WHERE server_id = $2"#, 
            show_changelogs, server_id)
            .execute(db)
            .await?;
        },
        None => {
            // Add server and set setting if it does not exist
            sqlx::query!(r#"INSERT INTO servers (server_id, show_changelog) VALUES ($1, $2)"#,
            server_id, show_changelogs)
            .execute(db)
            .await?;
        },
    };
    Ok(())
}

pub async fn add_mod_subscription(db: &Pool<Sqlite>, server_id: i64, modname: &str) -> Result<(), Error> {
    sqlx::query!(r#"INSERT OR REPLACE INTO subscribed_mods (server_id, mod_name) VALUES ($1, $2)"#, server_id, modname)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn remove_mod_subscription(db: &Pool<Sqlite>, server_id: i64, modname: &str) -> Result<(), Error> {
    sqlx::query!(r#"DELETE FROM subscribed_mods WHERE server_id = $1 AND mod_name = $2"#, server_id, modname)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn add_author_subscription(db: &Pool<Sqlite>, server_id: i64, author: &str) -> Result<(), Error> {
    sqlx::query!(r#"INSERT OR REPLACE INTO subscribed_authors (server_id, author_name) VALUES ($1, $2)"#, server_id, author)
        .execute(db)
        .await?;
    Ok(())
}

pub async fn remove_author_subscription(db: &Pool<Sqlite>, server_id: i64, author: &str) -> Result<(), Error> {
    sqlx::query!(r#"DELETE FROM subscribed_authors WHERE server_id = $1 AND author_name = $2"#, server_id, author)
        .execute(db)
        .await?;
    Ok(())
}

// pub async fn get_mod_data(db: &Pool<Sqlite>, modname: &str) -> Result<search_api::FoundMod, Error> {
//     let Ok(mod_data) = sqlx::query!(r#"SELECT * FROM mods WHERE name = $1"#, modname)
//         .fetch_one(db)
//         .await else {
//                 return Err(Box::new(CustomError::new( &format!("Failed to find mod {modname} in database"))));
//     };

//     let r = search_api::FoundMod{
//         downloads_count: mod_data.downloads_count,
//         name: mod_data.name.clone(),
//         owner: mod_data.owner,
//         summary: mod_data.summary.unwrap_or_default(),
//         thumbnail: update_notifications::get_mod_thumbnail(&mod_data.name).await.unwrap_or_else(|_| "https://assets-mod.factorio.com/assets/.thumb.png".to_owned()),
//         title: mod_data.title.unwrap_or_else(|| mod_data.name.clone()),
//         factorio_version: mod_data.factorio_version.unwrap_or_default(),
//     };
//     Ok(r)
// }

// pub async fn update_download_count(db: &Pool<Sqlite>, found_mod: &search_api::FoundMod, up_to_date: bool) -> Result<(), Error> {
//     let Ok(db_data) = sqlx::query!(r#"SELECT last_data_update FROM mods WHERE name = $1"#, found_mod.name)
//         .fetch_one(db)
//         .await else {
//             return Err(Box::new(CustomError::new( &format!("Failed to find mod {} in database", found_mod.name))));
//     };
//     if !up_to_date {
//         //call API to get current download count
//     }

//     let now = chrono::Utc::now().timestamp();
//     if now - db_data.last_data_update > 432_000 { // 5 days
//         sqlx::query!(r#"UPDATE mods SET downloads_count = $1, last_data_update = $2  WHERE name = $3"#, found_mod.downloads_count, now, found_mod.name)
//         .execute(db)
//         .await?;
//     };
//     Ok(())
// }

pub async fn get_last_mod_update_time(db: &Pool<Sqlite>, modname: &str) -> Result<Option<i64>, Error> {
    let record = sqlx::query!(r#"SELECT released_at FROM mods WHERE name = $1"#, modname)
        .fetch_optional(db)
        .await?;
    record.map_or_else(|| Ok(None), |rec| Ok(Some(rec.released_at)))
}

pub struct DBModEntry<'a> {
    pub name: &'a str,
    pub title: &'a str,
    pub owner: &'a str,
    pub summary: &'a str,
    pub category: &'a str,
    pub downloads_count: i32,
    pub factorio_version: &'a str,
    pub version: &'a str,
    pub released_at: i64,
}

pub async fn store_mod_data<'a>(db: &Pool<Sqlite>, mod_details: DBModEntry<'a>) -> Result<(), Error> {
    sqlx::query!(r#"INSERT OR REPLACE INTO mods 
        (name, title, owner, summary, category, downloads_count, factorio_version, version, released_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#, 
        mod_details.name,
        mod_details.title,
        mod_details.owner,
        mod_details.summary,
        mod_details.category,
        mod_details.downloads_count,
        mod_details.factorio_version,
        mod_details.version,
        mod_details.released_at,
    )
        .execute(db)
        .await?;
    Ok(())
}

#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
pub async fn get_mod_count(db: &Pool<Sqlite>) -> Result<i32, Error> {
    let record = sqlx::query!(r#"SELECT name FROM mods"#)
        .fetch_all(db)
        .await?;
    Ok(record.len() as i32)
}

pub async fn create_mods_cache(db: &Pool<Sqlite>) -> Result<Vec<ModCacheEntry>, Error> {
    let mod_cache = sqlx::query!(r#"
        SELECT name, title, owner, downloads_count, factorio_version 
        FROM mods 
        WHERE (factorio_version = $1 OR factorio_version = $2) 
        ORDER BY downloads_count DESC"#, "1.1", "2.0"
    )
        .fetch_all(db)
        .await?
        .iter()
        .map(|rec| {
            ModCacheEntry{
                name: rec.name.clone(),
                title: rec.title.clone().unwrap_or_default(), // Default if mod has no name (title)
                author: rec.owner.clone(),
                factorio_version: rec.factorio_version.clone().unwrap(), // Unwrap should be safe due to filters in sql query
            }
        })
        .collect::<Vec<ModCacheEntry>>();

    Ok(mod_cache)
}

pub async fn create_subscriptions_cache(db: &Pool<Sqlite>) -> Result<Vec<SubCacheEntry>, Error> {
    let mod_records = sqlx::query!(r#"SELECT * FROM subscribed_mods"#)
        .fetch_all(db)
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
                .fetch_all(db)
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
    Ok(mod_records)
}

pub async fn create_mod_author_cache(db: &Pool<Sqlite>) -> Result<Vec<String>, Error> {
    let mut author_records = sqlx::query!(r#"SELECT owner FROM mods"#)
        .fetch_all(db)
        .await?
        .into_iter()
        .map(|rec| rec.owner)
        .collect::<Vec<String>>();
    author_records.sort_unstable();
    author_records.dedup();
    Ok(author_records)
}