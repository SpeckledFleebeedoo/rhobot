use std::collections::HashMap;
use serde::Deserialize;
use crate::{
    custom_errors::CustomError, 
    Error, 
    formatting_tools::DiscordFormat,
};

pub struct ModPortalCredentials {
    username: String,
    token: String,
}

impl ModPortalCredentials {
    pub const fn new(username: String, token: String) -> Self {
        Self {username, token}
    }
}

#[derive(Deserialize, Debug, Clone)]
struct SearchApiResponse {
    results: Vec<FoundMod>
}

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct FoundMod {
    pub downloads_count: i64,
    pub name: String,
    pub owner: String,
    pub summary: String,
    pub thumbnail: String,
    pub title: String,
}

impl FoundMod {
    pub fn sanitize_for_embed(&mut self) {
        self.title = self.title
            .clone()
            .truncate_for_embed(256)
            .escape_formatting();
        self.summary = self.summary
            .clone()
            .truncate_for_embed(4096)
            .escape_formatting();
        self.owner = self.owner
            .clone()
            .truncate_for_embed(1024)
            .escape_formatting();
    }
}

pub async fn find_mod(name: &str, credentials: &ModPortalCredentials) -> Result<FoundMod, Error> {
    let mut name_truncated = name.to_owned();
    name_truncated.truncate(50);
    let map = HashMap::from([
        ("username", credentials.username.as_str()),
        ("token", credentials.token.as_str()),
        ("query", name_truncated.as_str()),
        ("version", "1.1"),
        ("sort_attribute", "relevancy"),
        ("only_bookmarks", "false"),
        ("show_deprecated", "false"),
        ("page", "1"),
        ("page_size", "1"),
        ("highlight_pre_tag", ""),
        ("highlight_post_tag", "")
    ]);

    let client = reqwest::Client::new();
    let response = client.post("https://mods.factorio.com/api/search")
        .json(&map)
        .send()
        .await?;
    match response.status() {
        reqwest::StatusCode::OK => (),
        _ => return Err(Box::new(CustomError::new(&format!("Received HTTP status code {} while accessing mod search API", response.status().as_str())))),
    };
    
    let found_mod_details = response.json::<SearchApiResponse>().await?;

    if found_mod_details.results.first().is_none() {
        return Err(Box::new(CustomError::new(&format!("Did not find any mods named {name}"))))
    };
    let mut mod_entry = found_mod_details.results.first().unwrap().to_owned();
    mod_entry.thumbnail = format!("https://assets-mod.factorio.com{}", mod_entry.thumbnail);
    Ok(mod_entry)
}