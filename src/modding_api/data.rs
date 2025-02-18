use serde::{Deserialize, Serialize};
use poise::serenity_prelude as serenity;
use poise::reply::CreateReply;
use std::{fmt, sync::{Arc, RwLock}};
use log::{error, info};

use crate::{
    formatting_tools::DiscordFormat, 
    Context, 
    Data, 
    Error
};

use super::{
    resolve_internal_links,
    split_inputs,
    error::ApiError,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BasicMember {
    pub name: String,
    pub order: i32,
    pub description: String,
    pub lists: Option<Vec<String>>,
    pub examples: Option<Vec<String>>,
    pub images: Option<Vec<Image>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ApiResponse {
    pub application: String,
    pub stage: String,
    pub application_version: String,
    pub api_version: i32,
    pub prototypes: Vec<Prototype>,
    pub types: Vec<DataStageType>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Prototype {
    #[serde(flatten)]
    pub common: BasicMember,
    pub visibility: Option<Vec<String>>,
    pub parent: Option<String>,
    pub r#abstract: bool,
    pub typename: Option<String>,
    pub instance_limit: Option<i32>,
    pub deprecated: bool,
    pub properties: Vec<Property>,
    pub custom_properties: Option<CustomProperties>,
}

#[allow(clippy::module_name_repetitions)]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataStageType {
    #[serde(flatten)]
    pub common: BasicMember,
    pub parent: Option<String>,
    pub r#abstract: bool,
    pub inline: bool,
    pub r#type: Type,
    pub properties: Option<Vec<Property>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Image {
    pub filename: String,
    pub caption: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Property {
    #[serde(flatten)]
    pub common: BasicMember,
    pub alt_name: Option<String>,
    pub r#override: bool,
    pub r#type: Type,
    pub optional: bool,
    pub default: Option<PropertyDefault>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum PropertyDefault {
    Type(ComplexType),
    String(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CustomProperties {
    pub description: String,
    pub lists: Option<Vec<String>>,
    pub examples: Option<Vec<String>>,
    pub images: Option<Vec<Image>>,
    pub key_type: Type,
    pub value_type: Type,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Type {
    Simple(String),
    Complex(Box<ComplexType>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "complex_type", rename_all = "snake_case")]
pub enum ComplexType {
    Type {value: Type, description: String },
    Union { options: Vec<Type>, full_format: bool },
    Array { value: Type },
    Dictionary { key: Type, value: Type },
    Literal { value: serde_json::Value, description: Option<String> },
    Tuple { values: Vec<Type> },
    Struct,
}

pub enum TypeOrPrototype<'a> {
    Type(&'a DataStageType),
    Prototype(&'a Prototype),
}

impl BasicMember {
    pub fn create_embed(&self, data: &Data) -> serenity::CreateEmbed {
        serenity::CreateEmbed::new()
            .title(&self.name)
            .description(resolve_internal_links(data, &self.description)
                .truncate_for_embed(4096)
            )
            .color(serenity::Colour::GOLD)
    }
}

impl Prototype {
    pub fn to_embed(&self, data: &Data) -> serenity::CreateEmbed {
        let url = format!("https://lua-api.factorio.com/latest/prototypes/{}.html", &self.common.name);
        self.common.create_embed(data)
        .author(serenity::CreateEmbedAuthor::new("Prototype")
            .url("https://lua-api.factorio.com/latest/prototypes.html"))
        .url(url)
    }
}

impl Property {
    pub fn to_embed(&self, data: &Data, parent: &TypeOrPrototype) -> serenity::CreateEmbed {
        match parent {
            TypeOrPrototype::Type(t) => {
                let url = format!("https://lua-api.factorio.com/latest/types/{}.html#{}", &t.common.name, &self.common.name);
                let optional = if self.optional {" (optional)"} else {""};
                let parent_name = &t.common.name;
                let t_name = &self.common.name;
                let description = format!("`{}{}`\n{}", &self.r#type, optional, resolve_internal_links(data, &self.common.description))
                    .truncate_for_embed(4096);

                serenity::CreateEmbed::new()
                    .title(format!("{parent_name}::{t_name}").truncate_for_embed(256))
                    .description(description)
                    .color(serenity::Colour::GOLD)
                    .url(url)
            },
            TypeOrPrototype::Prototype(p) => {
                let url = format!("https://lua-api.factorio.com/latest/prototypes/{}.html#{}", &p.common.name, &self.common.name);
                let optional = if self.optional {" (optional)"} else {""};
                let parent_name = &p.common.name;
                let p_name = &self.common.name;
                let description = format!("`{}{}`\n{}", &self.r#type, optional, resolve_internal_links(data, &self.common.description))
                    .truncate_for_embed(4096);

                serenity::CreateEmbed::new()
                    .title(format!("{parent_name}::{p_name}").truncate_for_embed(256))
                    .description(description)
                    .color(serenity::Colour::GOLD)
                    .url(url)
            },
        }
    }
}

impl DataStageType {
    pub fn to_embed(&self, data: &Data) -> serenity::CreateEmbed {
        let url = format!("https://lua-api.factorio.com/latest/types/{}.html", &self.common.name);
        self.common.create_embed(data)
        .title(format!("{} :: {}", &self.common.name, &self.r#type)) // Override name to include type
        .author(serenity::CreateEmbedAuthor::new("Type")
            .url("https://lua-api.factorio.com/latest/types.html"))
        .url(url)
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Simple(t) => write!(f, "{t}"),
            Self::Complex(ct) => write!(f, "{ct}"),
        }
    }
}

impl fmt::Display for ComplexType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Type { value, .. } => {write!(f, "{value}")},
            Self::Union { options, .. } => {
                let options_string = options.iter()
                    .map(|t| format!("{t}"))
                    .collect::<Vec<String>>()
                    .join(" or ");
                write!(f, "{options_string}")
            },
            Self::Array { value } => {write!(f, "array[{value}]")},
            Self::Dictionary { key, value } => {
                write!(f, "dictionary[{key} → {value}]")
            },
            Self::Literal { value, .. } => {
                match value {
                    serde_json::Value::String(str) => write!(f, r#""{}""#, &str),
                    serde_json::Value::Bool(bool) => write!(f, "{bool}"),
                    serde_json::Value::Number(num) => write!(f, "{num}"),
                    _ => write!(f, ""),
                }
            },
            Self::Tuple { .. } => write!(f, "tuple"),
            Self::Struct => write!(f, "struct"),
        }
    }
}

pub async fn update_api_cache(
    cache: Arc<RwLock<ApiResponse>>,
) -> Result<(), Error> {
    info!("Updating data stage API cache");
    let new_data_api = get_data_api().await?;
    match cache.write() {
        Ok(mut c) => *c = new_data_api,
        Err(e) => {
            return Err(ApiError::CacheError(e.to_string()))?;
        },
    };
    Ok(())
}

pub async fn get_data_api() -> Result<ApiResponse, Error> {
    let response = reqwest::get("https://lua-api.factorio.com/latest/prototype-api.json").await?;
    match response.status() {
        reqwest::StatusCode::OK => (),
        _ => return Err(ApiError::BadStatusCode(response.status().to_string()))?
    };
    Ok(response.json::<ApiResponse>().await?)
}

/// Link a modding API prototype
#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, track_edits, rename="prototype", install_context = "Guild|User", interaction_context = "Guild|BotDm|PrivateChannel")]
pub async fn api_prototype (
    ctx: Context<'_>,
    #[description = "Search term"]
    #[autocomplete = "autocomplete_prototype"]
    #[rename = "prototype"]
    mut prototype_search: String,
    #[description = "Prototype property"]
    #[autocomplete = "autocomplete_prototype_property"]
    #[rename = "property"]
    #[rest]
    mut property_search: Option<String>,
) -> Result<(), Error> {
    let cache = ctx.data().data_api_cache.clone();
    let api = match cache.read() {
        Ok(c) => c,
        Err(e) => {
            return Err(ApiError::CacheError(e.to_string()))?
        },
    }.clone();

    split_inputs(&mut prototype_search, &mut property_search);

    let Some(search_result) = api.prototypes.iter()
        .find(|p| prototype_search.eq_ignore_ascii_case(&p.common.name)) 
    else {
        return Err(ApiError::PrototypeNotFound(prototype_search))?;
    };

    let embed = if let Some(property_name) = property_search {
        make_property_embed(&TypeOrPrototype::Prototype(search_result), &property_name, ctx)?
    } else {
        search_result.to_embed(ctx.data())
    };

    let builder = CreateReply::default()
        .embed(embed);
    ctx.send(builder).await?;
    Ok(())
}

#[allow(clippy::unused_async)]
async fn autocomplete_prototype<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let cache = ctx.data().data_api_cache.clone();
    let api = match cache.read(){
        Ok(c) => c,
        Err(e) => {
            error!{"Error acquiring cache: {e}"}
            return vec![]
        },
    }.clone();
    api.prototypes.iter()
        .filter(|p| p.common.name.to_lowercase().contains(&partial.to_lowercase()))
        .map(|p| p.common.name.clone())
        .collect::<Vec<String>>()
}

#[allow(clippy::unused_async)]
async fn autocomplete_prototype_property<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let poise::Context::Application(appcontext) = ctx else {return vec![]};
    let serenity::ResolvedValue::String(prototype_name) = appcontext.args[0].value else {return vec![]};
    if prototype_name.is_empty() {
        return vec![];
    };

    let cache = ctx.data().data_api_cache.clone();
    let api = match cache.read(){
        Ok(c) => c,
        Err(e) => {
            error!{"Error acquiring cache: {e}"}
            return vec![]
        },
    }.clone();

    let Some(prototype) = api.prototypes.iter()
        .find(|p| p.common.name.eq_ignore_ascii_case(prototype_name)) 
    else {return vec![]};    // Happens when invalid class is used

    prototype.properties.clone()
        .into_iter()
        .map(|p| p.common.name)
        .filter(|n| n.to_lowercase().contains(&partial.to_lowercase()))
        .collect::<Vec<String>>()
}

/// Link a modding API type
#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, track_edits, rename="type", install_context = "Guild|User", interaction_context = "Guild|BotDm|PrivateChannel")]
pub async fn api_type (
    ctx: Context<'_>,
    #[description = "Search term"]
    #[autocomplete = "autocomplete_type"]
    #[rename = "type"]
    mut type_search: String,
    #[description = "Type property"]
    #[autocomplete = "autocomplete_type_property"]
    #[rename = "property"]
    #[rest]
    mut property_search: Option<String>,
) -> Result<(), Error> {
    let cache = ctx.data().data_api_cache.clone();
    let api = match cache.read(){
        Ok(c) => c,
        Err(e) => {
            return Err(ApiError::CacheError(e.to_string()))?
        },
    }.clone();

    split_inputs(&mut type_search, &mut property_search);

    let Some(search_result) = api.types.iter()
        .find(|t| type_search.eq_ignore_ascii_case(&t.common.name)) 
        else {
            return Err(ApiError::TypeNotFound(type_search))?;
        };
    
    let embed = if let Some(property_name) = property_search {
        make_property_embed(&TypeOrPrototype::Type(search_result), &property_name, ctx)?
    } else {
        search_result.to_embed(ctx.data())
    };

    let builder = CreateReply::default()
        .embed(embed);
    ctx.send(builder).await?;
    Ok(())
}

#[allow(clippy::option_if_let_else)]
fn make_property_embed(search_result: &TypeOrPrototype, property_name: &str, ctx: Context<'_>) ->Result<serenity::CreateEmbed, Error> {
    let properties = match search_result {
        TypeOrPrototype::Prototype(pt) => pt.properties.clone(),
        TypeOrPrototype::Type(t) => {
            t.properties.clone().ok_or(ApiError::NoTypeProperties)?
        },
    };

    let property = properties
        .iter()
        .find(|m| m.common.name.eq_ignore_ascii_case(property_name));
    
    if let Some(p) = property {
        Ok(p.to_embed(ctx.data(), search_result))
    } else {
        Err(ApiError::PropertyNotFound(property_name.to_string()))?
    }
}

#[allow(clippy::unused_async)]
async fn autocomplete_type<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let cache = ctx.data().data_api_cache.clone();
    let api = match cache.read(){
        Ok(c) => c,
        Err(e) => {
            error!{"Error acquiring cache: {e}"}
            return vec![]
        },
    }.clone();
    api.types.iter()
        .filter(|p| p.common.name.to_lowercase().contains(&partial.to_lowercase()))
        .map(|p| p.common.name.clone())
        .collect::<Vec<String>>()
}

#[allow(clippy::unused_async)]
async fn autocomplete_type_property<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let poise::Context::Application(appcontext) = ctx else {return vec![]};
    let serenity::ResolvedValue::String(type_name) = appcontext.args[0].value else {return vec![]};
    if type_name.is_empty() {
        return vec![];
    };

    let cache = ctx.data().data_api_cache.clone();
    let api = match cache.read(){
        Ok(c) => c,
        Err(e) => {
            error!{"Error acquiring cache: {e}"}
            return vec![]
        },
    }.clone();

    let Some(datatype) = api.types.iter()
        .find(|p| p.common.name.eq_ignore_ascii_case(type_name)) 
    else {return vec![]};

    datatype.properties.as_ref().map_or_else(Vec::new, |properties| properties
        .iter()
        .map(|p| p.common.name.clone())
        .filter(|n| n.to_lowercase().contains(&partial.to_lowercase()))
        .collect::<Vec<String>>())
}

#[allow(unused_imports)]
mod tests {

    use super::*;
    use std::io::Read;
    
    #[tokio::test]
    async fn decode_api() {
        let file = std::fs::File::open("prototype-api-v5.json");
        assert!(file.is_ok(), "Failed to read file");

        let buf_reader = std::io::BufReader::new(file.unwrap());
        let api_data: Result<ApiResponse, serde_json::Error> = serde_json::from_reader(buf_reader);
        match api_data {
            Ok(_) => {},
            Err(e) => {panic!("{}", e)}
        };
    }
}
