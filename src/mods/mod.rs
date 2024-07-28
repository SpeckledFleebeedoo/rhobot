pub mod commands;
pub mod update_notifications;
pub mod search_api;

use sqlx::{Pool, Sqlite};
use crate::Error;

#[allow(clippy::module_name_repetitions)]
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