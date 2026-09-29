use tracing::{error, info};
use poise::reply::CreateReply;
use poise::serenity_prelude as serenity;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    sync::{Arc, RwLock},
};

use crate::{Context, Data, Error, formatting_tools::DiscordFormat};

use super::{error::ApiError, resolve_internal_links};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BasicMember {
    pub name: String,
    pub order: i32,
    pub description: String,
    pub lists: Option<Vec<String>>,
    pub examples: Option<Vec<String>>,
    pub images: Option<Vec<Image>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ApiResponse {
    pub application: String,
    pub stage: String,
    pub application_version: String,
    pub api_version: i32,
    pub prototypes: Vec<Prototype>,
    pub types: Vec<DataStageType>,
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
    Type {
        value: Type,
        description: String,
    },
    Union {
        options: Vec<Type>,
        full_format: bool,
    },
    Array {
        value: Type,
    },
    Dictionary {
        key: Type,
        value: Type,
    },
    Literal {
        value: serde_json::Value,
        description: Option<String>,
    },
    Tuple {
        values: Vec<Type>,
    },
    Struct,
}

pub enum TypeOrPrototype<'a> {
    Type(&'a DataStageType),
    Prototype(&'a Prototype),
}

impl BasicMember {
    pub fn create_embed(&self, data: &Data) -> serenity::CreateEmbed<'_> {
        serenity::CreateEmbed::new()
            .title(&self.name)
            .description(resolve_internal_links(data, &self.description).truncate_for_embed(4096))
            .color(serenity::Colour::GOLD)
    }
}

impl Prototype {
    pub fn to_embed(&self, data: &Data) -> serenity::CreateEmbed<'_> {
        let url = format!(
            "https://lua-api.factorio.com/latest/prototypes/{}.html",
            self.common.name
        );
        self.common
            .create_embed(data)
            .author(
                serenity::CreateEmbedAuthor::new("Prototype")
                    .url("https://lua-api.factorio.com/latest/prototypes.html"),
            )
            .url(url)
    }
}

impl Property {
    pub fn to_embed(&self, data: &Data, parent: &TypeOrPrototype) -> serenity::CreateEmbed<'_> {
        match parent {
            TypeOrPrototype::Type(t) => {
                let url = format!(
                    "https://lua-api.factorio.com/latest/types/{}.html#{}",
                    t.common.name, self.common.name
                );
                let optional = if self.optional { " (optional)" } else { "" };
                let parent_name = &t.common.name;
                let t_name = &self.common.name;
                let description = format!(
                    "`{}{}`\n{}",
                    self.r#type,
                    optional,
                    resolve_internal_links(data, &self.common.description)
                )
                .truncate_for_embed(4096);

                serenity::CreateEmbed::new()
                    .title(format!("{parent_name}::{t_name}").truncate_for_embed(256))
                    .description(description)
                    .color(serenity::Colour::GOLD)
                    .url(url)
            }
            TypeOrPrototype::Prototype(p) => {
                let url = format!(
                    "https://lua-api.factorio.com/latest/prototypes/{}.html#{}",
                    p.common.name, self.common.name
                );
                let optional = if self.optional { " (optional)" } else { "" };
                let parent_name = &p.common.name;
                let p_name = &self.common.name;
                let description = format!(
                    "`{}{}`\n{}",
                    self.r#type,
                    optional,
                    resolve_internal_links(data, &self.common.description)
                )
                .truncate_for_embed(4096);

                serenity::CreateEmbed::new()
                    .title(format!("{parent_name}::{p_name}").truncate_for_embed(256))
                    .description(description)
                    .color(serenity::Colour::GOLD)
                    .url(url)
            }
        }
    }
}

impl DataStageType {
    pub fn to_embed(&self, data: &Data) -> serenity::CreateEmbed<'_> {
        let url = format!(
            "https://lua-api.factorio.com/latest/types/{}.html",
            self.common.name
        );
        self.common
            .create_embed(data)
            .title(format!("{} :: {}", self.common.name, self.r#type)) // Override name to include type
            .author(
                serenity::CreateEmbedAuthor::new("Type")
                    .url("https://lua-api.factorio.com/latest/types.html"),
            )
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
            Self::Type { value, .. } => {
                write!(f, "{value}")
            }
            Self::Union { options, .. } => {
                if options.len() > 6 {
                    return write!(f, "union")
                }
                let options_string = options
                    .iter()
                    .map(|t| format!("{t}"))
                    .collect::<Vec<String>>()
                    .join(" or ");
                write!(f, "{options_string}")
            }
            Self::Array { value } => {
                write!(f, "array[{value}]")
            }
            Self::Dictionary { key, value } => {
                write!(f, "dictionary[{key} → {value}]")
            }
            Self::Literal { value, .. } => match value {
                serde_json::Value::String(str) => write!(f, r#""{str}""#),
                serde_json::Value::Bool(bool) => write!(f, "{bool}"),
                serde_json::Value::Number(num) => write!(f, "{num}"),
                _ => write!(f, ""),
            },
            Self::Tuple { .. } => write!(f, "tuple"),
            Self::Struct => write!(f, "struct"),
        }
    }
}

pub async fn update_api_cache(cache: Arc<RwLock<ApiResponse>>) -> Result<(), Error> {
    info!("Updating data stage API cache");
    let new_data_api = get_data_api().await?;
    match cache.write() {
        Ok(mut c) => *c = new_data_api,
        Err(e) => {
            Err(ApiError::CacheError(e.to_string()))?;
        }
    }
    Ok(())
}

pub async fn get_data_api() -> Result<ApiResponse, Error> {
    let url = "https://lua-api.factorio.com/latest/prototype-api.json";
    let user_agent = std::env::var("USER_AGENT").unwrap_or_else(|_| "Rhobot".to_string());
    let client = reqwest::Client::builder()
        .user_agent(user_agent)
        .build()
        .map_err(ApiError::from)?;
    let response = client.get(url).send().await.map_err(ApiError::from)?;
    match response.status() {
        reqwest::StatusCode::OK => (),
        _ => return Err(ApiError::BadStatusCode(response.status().to_string()))?,
    }
    Ok(response
        .json::<ApiResponse>()
        .await
        .map_err(ApiError::from)?)
}

pub fn generate_autocomplete_cache(api_cache: &Arc<RwLock<ApiResponse>>, autocomplete_cache: &Arc<RwLock<Vec<AutocompleteOption>>>) -> Result<(), Error> {
    let api = match api_cache.read() {
        Ok(c) => c,
        Err(e) => return Err(ApiError::CacheError(e.to_string()))?,
    }.clone();

    let data_autocomplete = api.prototypes.iter()
        .flat_map(|p| {
            std::iter::once(AutocompleteOption {
                kind: AutocompleteType::Prototype,
                search: p.common.name.clone(),
                parent: None,
            })
            .chain(p.properties.iter()
                .map(|pr| AutocompleteOption {
                    kind: AutocompleteType::Prototype,
                    search: pr.common.name.clone(),
                    parent: Some(p.common.name.clone())
            }))
        })
        
        .chain(api.types.iter()
        .flat_map(|t| {
            std::iter::once(AutocompleteOption {
                kind: AutocompleteType::Type,
                search: t.common.name.clone(),
                parent: None,
            })
            .chain(t.properties.iter().flatten()
                .map(|tp| AutocompleteOption {
                    kind: AutocompleteType::Type,
                    search: tp.common.name.clone(),
                    parent: Some(t.common.name.clone()),
            }))
    }))
    .collect::<Vec<AutocompleteOption>>();

    let mut c = match autocomplete_cache.write() {
        Ok(c) => c,
        Err(e) => return Err(ApiError::CacheError(e.to_string()))?,
    };
    *c = data_autocomplete;
    drop(c);
    Ok(())
}

pub async fn api_prototype(
    ctx: Context<'_>,
    prototype_search: &str,
    property_search: Option<&str>,
) -> Result<(), Error> {
    let cache = ctx.data().data_api_cache.clone();
    let api = match cache.read() {
        Ok(c) => c,
        Err(e) => return Err(ApiError::CacheError(e.to_string()))?,
    }
    .clone();

    let Some(search_result) = api
        .prototypes
        .iter()
        .find(|p| prototype_search.eq_ignore_ascii_case(&p.common.name))
    else {
        return Err(ApiError::PrototypeNotFound(prototype_search.to_string()))?;
    };

    let t= TypeOrPrototype::Prototype(search_result);
    let embed = if let Some(property_name) = property_search {
        make_property_embed(
            &t,
            property_name,
            ctx,
        )?
    } else {
        search_result.to_embed(&ctx.data())
    };

    let builder = CreateReply::default()
        .embed(embed)
        .reply(true)
        .allowed_mentions(serenity::CreateAllowedMentions::default());
    ctx.send(builder).await?;
    Ok(())
}

pub async fn api_type(
    ctx: Context<'_>,
    type_search: &str,
    property_search: Option<&str>,
) -> Result<(), Error> {
    let cache = ctx.data().data_api_cache.clone();
    let api = match cache.read() {
        Ok(c) => c,
        Err(e) => return Err(ApiError::CacheError(e.to_string()))?,
    }
    .clone();

    let Some(search_result) = api
        .types
        .iter()
        .find(|t| type_search.eq_ignore_ascii_case(&t.common.name))
    else {
        return Err(ApiError::TypeNotFound(type_search.to_string()))?;
    };
    let t = TypeOrPrototype::Type(search_result);
    let embed = if let Some(property_name) = property_search {
        make_property_embed(&t, property_name, ctx)?
    } else {
        search_result.to_embed(&ctx.data())
    };

    let builder = CreateReply::default()
        .embed(embed)
        .reply(true)
        .allowed_mentions(serenity::CreateAllowedMentions::default());
    ctx.send(builder).await?;
    Ok(())
}

#[allow(clippy::option_if_let_else)]
fn make_property_embed<'a>(
    search_result: &'a TypeOrPrototype<'a>,
    property_name: &'a str,
    ctx: Context<'a>,
) -> Result<serenity::CreateEmbed<'a>, Error> {
    let property = match search_result {
        TypeOrPrototype::Prototype(pt) => pt
            .properties
            .iter()
            .find(|m| m.common.name.eq_ignore_ascii_case(property_name)),
        TypeOrPrototype::Type(t) => t
            .properties
            .as_ref()
            .ok_or(ApiError::NoTypeProperties)?
            .iter()
            .find(|m| m.common.name.eq_ignore_ascii_case(property_name)),
    };

    if let Some(p) = property {
        Ok(p.to_embed(&ctx.data(), search_result))
    } else {
        Err(ApiError::PropertyNotFound(property_name.to_string()))?
    }
}

/// Link to the data stage modding API
#[allow(clippy::unused_async)]
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn prototype(
    ctx: Context<'_>,
    #[description = "Search term"]
    #[autocomplete = "autocomplete_datastage"]
    search: String,
) -> Result<(), Error> {
    if search.starts_with("[P] ") {
        let searchtext = search.trim_start_matches("[P] ");
        if let Some((prototype, property)) = searchtext.split_once("::") {
            api_prototype(ctx, prototype, Some(property)).await?;
        } else {
            api_prototype(ctx, searchtext, None).await?;
        }
    } else if search.starts_with("[T] ") {
        let searchtext = search.trim_start_matches("[T] ");
        if let Some((apitype, property)) = searchtext.split_once("::") {
            api_type(ctx, apitype, Some(property)).await?;
        } else {
            api_type(ctx, searchtext, None).await?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
enum AutocompleteType {
    Prototype,
    Type,
}

#[derive(Debug, Clone)]
pub struct AutocompleteOption {
    kind: AutocompleteType,
    search: String,
    parent: Option<String>
}

impl fmt::Display for AutocompleteOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            AutocompleteType::Prototype => write!(f, "[P] "),
            AutocompleteType::Type => write!(f, "[T] "),
        }?;
        if let Some(parent) = &self.parent {
            write!(f, "{}::{}", parent, self.search)
        } else {
            write!(f, "{}", self.search)
        }
    }
}

#[allow(clippy::unused_async)]
async fn autocomplete_datastage<'a>(ctx: Context<'a>, partial: &'a str) -> serenity::CreateAutocompleteResponse<'a> {
    let cache = ctx.data().data_api_autocomplete_cache.clone();
    let data_autocomplete = match cache.read() {
        Ok(c) => c,
        Err(e) => {
            error! {"Error acquiring cache: {e}"}
            return serenity::CreateAutocompleteResponse::new();
        }
    }
    .clone();

    let choices = data_autocomplete.iter()
        .filter(|t| t.search.to_lowercase().contains(partial.to_lowercase().trim()))
        .take(25)
        .map(|t| 
            serenity::AutocompleteChoice::from(format!("{t}")))
        .collect::<Vec<serenity::AutocompleteChoice>>();

    serenity::CreateAutocompleteResponse::new().set_choices(choices)
}

#[allow(unused_imports)]
mod tests {

    use super::*;
    use std::io::Read;

    #[tokio::test]
    async fn decode_api() {
        let file = std::fs::File::open("prototype-api-v6.json");
        assert!(file.is_ok(), "Failed to read file");

        let buf_reader = std::io::BufReader::new(file.unwrap());
        let api_data: Result<ApiResponse, serde_json::Error> = serde_json::from_reader(buf_reader);
        match api_data {
            Ok(_) => {}
            Err(e) => {
                panic!("{}", e)
            }
        }
    }
}
