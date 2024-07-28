use serde::{Deserialize, Serialize};
use poise::serenity_prelude as serenity;
use poise::reply::CreateReply;
use std::{fmt, sync::{Arc, RwLock}};
use log::{error, info};

use crate::{
    Context, 
    custom_errors::CustomError, 
    Data, 
    Error, 
    modding_api::resolve_internal_links, 
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

impl BasicMember {
    pub fn create_embed(&self, data: &Data) -> serenity::CreateEmbed {
        serenity::CreateEmbed::new()
            .title(&self.name)
            .description(resolve_internal_links(data, &self.description))
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
    cache: Arc<RwLock<ApiResponse>>,
) -> Result<(), Error> {
    info!("Updating data stage API cache");
    let new_data_api = get_data_api().await?;
    match cache.write() {
        Ok(mut c) => *c = new_data_api,
        Err(e) => {
            return Err(Box::new(CustomError::new(&format!("Error acquiring cache: {e}"))));
        },
    };
    Ok(())
}

pub async fn get_data_api() -> Result<ApiResponse, Error> {
    let response = reqwest::get("https://lua-api.factorio.com/latest/prototype-api.json").await?;
    match response.status() {
        reqwest::StatusCode::OK => (),
        _ => return Err(Box::new(CustomError::new(&format!("Received HTTP status code {} while accessing Lua prototype API", response.status().as_str()))))
    };
    Ok(response.json::<ApiResponse>().await?)
}

#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, track_edits, rename="prototype")]
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
    let api = match cache.read() {
        Ok(c) => c,
        Err(e) => {
            return Err(Box::new(CustomError::new(&format!("Error acquiring cache: {e}"))));
        },
    }.clone();
    
    let Some(search_result) = api.prototypes.iter()
        .find(|p| prototype_search.eq_ignore_ascii_case(&p.common.name)) 
    else {
        return Err(Box::new(CustomError::new(&format!("Could not find prototype `{prototype_search}` in API documentation"))));
    };
    let mut embed = search_result.to_embed(ctx.data());

    if let Some(property_name) = property_search {
        let property = search_result.properties.clone()
            .into_iter()
            .find(|m| m.common.name.eq_ignore_ascii_case(&property_name));

        if let Some(p) = property {
            let optional = if p.optional {"optional"} else {""};
            let prototype_name = &search_result.common.name;
            let p_name = &p.common.name;
            let p_type = &p.r#type;
            let description = resolve_internal_links(ctx.data(), &p.common.description);
            embed = embed.field(
                format!("`{p_name} {optional} :: {p_type}`"), 
                format!("{description}\n[Full documentation](https://lua-api.factorio.com/latest/prototypes/{prototype_name}.html#{p_name})"), 
                false
            );
        } else {
            embed = embed.field("Error", format!("Could not find property `{property_name}`"), false);
        };
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
        .filter(|p| p.common.name.to_lowercase().starts_with(&partial.to_lowercase()))
        .map(|p| p.common.name.clone())
        .collect::<Vec<String>>()
}

#[allow(clippy::unused_async)]
async fn autocomplete_prototype_property<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let mut prototype_name = String::new();
    if let poise::Context::Application(appcontext) = ctx {
        if let serenity::ResolvedValue::String(st) = appcontext.args[0].value {
            st.clone_into(&mut prototype_name);
        }
    }

    if prototype_name.is_empty() {
        return vec![];   // Happens when property field is used before class field
    }
    let cache = ctx.data().data_api_cache.clone();
    let api = match cache.read(){
        Ok(c) => c,
        Err(e) => {
            error!{"Error acquiring cache: {e}"}
            return vec![]
        },
    }.clone();

    let Some(prototype) = api.prototypes.iter()
        .find(|p| p.common.name.eq_ignore_ascii_case(&prototype_name)) 
    else {return vec![]};    // Happens when invalid class is used

    prototype.properties.clone()
        .into_iter()
        .map(|p| p.common.name)
        .filter(|n| n.to_lowercase().contains(&partial.to_lowercase()))
        .collect::<Vec<String>>()
}

#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, track_edits, rename="type")]
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
    let api = match cache.read(){
        Ok(c) => c,
        Err(e) => {
            return Err(Box::new(CustomError::new(&format!("Error acquiring cache: {e}"))));
        },
    }.clone();
    let Some(search_result) = api.types.iter()
        .find(|t| type_search.eq_ignore_ascii_case(&t.common.name)) 
        else {
            return Err(Box::new(CustomError::new(&format!("Could not find type `{type_search}` in API documentation"))));
        };

    let mut embed = search_result.to_embed(ctx.data());

    if let Some(property_name) = property_search {
        if let Some(properties) = &search_result.properties {
            let property = properties
                .iter()
                .find(|m| m.common.name.eq_ignore_ascii_case(&property_name));

            if let Some(p) = property {             // name optional  :: type    Description
                let optional = if p.optional {"optional"} else {""};
                let type_name = &search_result.common.name;
                let p_name = &p.common.name;
                let p_type = &p.r#type;
                let description = resolve_internal_links(ctx.data(), &p.common.description);
                embed = embed.field(
                    format!("`{p_name} {optional} :: {p_type}`"), 
                    format!("{description}\n[Full documentation](https://lua-api.factorio.com/latest/types/{type_name}.html#{p_name})"), 
                    false
                );
            };
        } else {
            embed = embed.field("Error", format!("Could not find property `{property_name}`"), false);
        };
    };
    let builder = CreateReply::default()
        .embed(embed);
    ctx.send(builder).await?;
    Ok(())
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
        .filter(|p| p.common.name.to_lowercase().starts_with(&partial.to_lowercase()))
        .map(|p| p.common.name.clone())
        .collect::<Vec<String>>()
}

#[allow(clippy::unused_async)]
async fn autocomplete_type_property<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let mut type_name = String::new();
    if let poise::Context::Application(appcontext) = ctx {
        if let serenity::ResolvedValue::String(st) = appcontext.args[0].value {
            st.clone_into(&mut type_name);
        }
    }

    if type_name.is_empty() {
        return vec![];   // Happens when property field is used before class field
    }
    let cache = ctx.data().data_api_cache.clone();
    let api = match cache.read(){
        Ok(c) => c,
        Err(e) => {
            error!{"Error acquiring cache: {e}"}
            return vec![]
        },
    }.clone();

    let Some(datatype) = api.types.iter()
        .find(|p| p.common.name.eq_ignore_ascii_case(&type_name)) 
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
