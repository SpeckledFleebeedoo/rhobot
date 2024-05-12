use serde::{Deserialize, Serialize};
use poise::serenity_prelude as serenity;
use poise::reply::CreateReply;
use std::{fmt, sync::{Arc, RwLock}};

use crate::{Context, Error, custom_errors::CustomError};


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BasicMember {
    pub name: String,
    pub order: i32,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataApiResponse {
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
    pub lists: Option<Vec<String>>,
    pub examples: Option<Vec<String>>,
    pub images: Option<Vec<Image>>,
    pub parent: Option<String>,
    pub r#abstract: bool,
    pub typename: Option<String>,
    pub instance_limit: Option<i32>,
    pub deprecated: bool,
    pub properties: Vec<Property>,
    pub custom_properties: Option<CustomProperties>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataStageType {
    #[serde(flatten)]
    pub common: BasicMember,
    pub lists: Option<Vec<String>>,
    pub examples: Option<Vec<String>>,
    pub images: Option<Vec<Image>>,
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
    pub lists: Option<Vec<String>>,
    pub examples: Option<Vec<String>>,
    pub images: Option<Vec<Image>>,
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

impl BasicMember {
    pub async fn create_embed(&self, url: &str) -> serenity::CreateEmbed {
        serenity::CreateEmbed::new()
            .title(&self.name)
            .description(&self.description)
            .url(url)
            .color(serenity::Colour::GOLD)
    }
}

impl Prototype {
    pub async fn to_embed(&self) -> serenity::CreateEmbed {
        let url = format!("https://lua-api.factorio.com/latest/defines.html#defines.{}", &self.common.name);
        self.common.create_embed(&url).await
    }
}

impl DataStageType {
    pub async fn to_embed(&self) -> serenity::CreateEmbed {
        let url = format!("https://lua-api.factorio.com/latest/defines.html#defines.{}", &self.common.name);
        self.common.create_embed(&url).await
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
                write!(f, "dictionary[{key} 🡪 {value}]")
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
    cache: Arc<RwLock<DataApiResponse>>,
) -> Result<(), Error> {
    println!("Updating data API cache");
    {
    let new_data_api = get_data_api().await?;
    let mut c = cache.write().unwrap();
    *c = new_data_api;
    }
    Ok(())
}

pub async fn get_data_api() -> Result<DataApiResponse, Error> {
    let response = reqwest::get("https://lua-api.factorio.com/latest/prototype-api.json").await?;
    match response.status() {
        reqwest::StatusCode::OK => (),
        _ => return Err(Box::new(CustomError::new("Received HTTP status code that is not 200")))
    };
    Ok(response.json::<DataApiResponse>().await?)
}

#[poise::command(prefix_command, slash_command, guild_only, subcommands("api_prototype", "api_type"), rename="data")]
pub async fn api_data(
    _ctx: Context<'_>
) -> Result<(), Error> {
    Ok(())
}

#[poise::command(prefix_command, slash_command, rename="prototype")]
pub async fn api_prototype (
    ctx: Context<'_>,
    #[description = "Search term"]
    #[autocomplete = "autocomplete_prototype"]
    #[rename = "prototype"]
    prototype_search: String,
    #[description = "Prototype property"]
    #[autocomplete = "autocomplete_prototype_property"]
    #[rename = "property"]
    property_search: Option<String>,
) -> Result<(), Error> {
    let cache = ctx.data().data_api_cache.clone();
    let api = cache.read().unwrap().clone();
    let search_result = api.prototypes.iter()
        .find(|prototype| prototype_search.eq_ignore_ascii_case(&prototype.common.name)).unwrap();
    let mut embed = search_result.to_embed().await;

    if let Some(property_name) = property_search {
        let property = search_result.properties.clone()
            .into_iter()
            .find(|m| m.common.name == property_name);

        if let Some(p) = property {             // name optional  :: type    Description
            let optional = if p.optional {"optional"} else {""};
            embed = embed.field(format!("`{} {} :: {}`", p.common.name, optional, p.r#type), p.common.description, false);
        };
    };
    let builder = CreateReply::default()
        .embed(embed);
    ctx.send(builder).await?;
    Ok(())
}

async fn autocomplete_prototype<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let cache = ctx.data().data_api_cache.clone();
    let api = cache.read().unwrap().clone();
    api.prototypes.iter()
        .filter(|p| p.common.name.to_lowercase().starts_with(&partial.to_lowercase()))
        .map(|p| p.common.name.clone())
        .collect::<Vec<String>>()
}

async fn autocomplete_prototype_property<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let mut prototype_name = String::new();
    if let poise::Context::Application(appcontext) = ctx {
        if let serenity::ResolvedValue::String(st) = appcontext.args[0].value {
            prototype_name = st.to_owned();
        }
    }

    if prototype_name.is_empty() {
        return vec![];   // Happens when property field is used before class field
    }
    let cache = ctx.data().data_api_cache.clone();
    let api = cache.read().unwrap().clone();
    let prototype = match api.prototypes.iter()
        .find(|p| p.common.name == prototype_name) {
            Some(p) => p,
            None => {return vec![]},    // Happens when invalid class is used
        };
    prototype.properties.clone()
        .into_iter()
        .map(|p| p.common.name)
        .filter(|n| n.to_lowercase().contains(&partial.to_lowercase()))
        .collect::<Vec<String>>()
}

#[poise::command(prefix_command, slash_command, rename="type")]
pub async fn api_type (
    ctx: Context<'_>,
    #[description = "Search term"]
    #[autocomplete = "autocomplete_type"]
    #[rename = "type"]
    type_search: String,
    #[description = "Type property"]
    #[autocomplete = "autocomplete_type_property"]
    #[rename = "property"]
    property_search: Option<String>,
) -> Result<(), Error> {
    let cache = ctx.data().data_api_cache.clone();
    let api = cache.read().unwrap().clone();
    let search_result = api.types.iter()
        .find(|datatype| type_search.eq_ignore_ascii_case(&datatype.common.name)).unwrap();
    let mut embed = search_result.to_embed().await;

    if let Some(property_name) = property_search {
        if let Some(properties) = &search_result.properties {
            let property = properties
                .iter()
                .find(|m| m.common.name == property_name);

            if let Some(p) = property {             // name optional  :: type    Description
                let optional = if p.optional {"optional"} else {""};
                embed = embed.field(format!("`{} {} :: {}`", p.common.name, optional, p.r#type), p.common.description.clone(), false);
            };
        };
    };
    let builder = CreateReply::default()
        .embed(embed);
    ctx.send(builder).await?;
    Ok(())
}

async fn autocomplete_type<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let cache = ctx.data().data_api_cache.clone();
    let api = cache.read().unwrap().clone();
    api.types.iter()
        .filter(|p| p.common.name.to_lowercase().starts_with(&partial.to_lowercase()))
        .map(|p| p.common.name.clone())
        .collect::<Vec<String>>()
}

async fn autocomplete_type_property<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let mut type_name = String::new();
    if let poise::Context::Application(appcontext) = ctx {
        if let serenity::ResolvedValue::String(st) = appcontext.args[0].value {
            type_name = st.to_owned();
        }
    }

    if type_name.is_empty() {
        return vec![];   // Happens when property field is used before class field
    }
    let cache = ctx.data().data_api_cache.clone();
    let api = cache.read().unwrap().clone();
    let datatype = match api.types.iter()
        .find(|p| p.common.name == type_name) {
            Some(p) => p,
            None => {return vec![]},    // Happens when invalid class is used
        };
    if let Some(properties) = &datatype.properties {
    properties
        .iter()
        .map(|p| p.common.name.clone())
        .filter(|n| n.to_lowercase().contains(&partial.to_lowercase()))
        .collect::<Vec<String>>()
    } else {vec![]}
}