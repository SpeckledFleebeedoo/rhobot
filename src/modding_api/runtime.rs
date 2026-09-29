use poise::reply::CreateReply;
use poise::serenity_prelude as serenity;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    sync::{Arc, RwLock},
};
use tracing::error;

use crate::{Context, Data, Error, formatting_tools::DiscordFormat};

use super::{error::ApiError, resolve_internal_links};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct BasicMember {
    pub name: String,
    pub order: i32,
    pub description: String,
    pub lists: Option<Vec<String>>,
    pub examples: Option<Vec<String>>,
    pub images: Option<Vec<Image>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Image {
    pub filename: String,
    caption: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct ApiResponse {
    pub application: String,
    pub application_version: String,
    pub api_version: i32,
    pub stage: String,
    pub classes: Vec<Class>,
    pub events: Vec<Event>,
    pub defines: Vec<Define>,
    pub concepts: Vec<Concept>,
    pub global_objects: Vec<GlobalObject>,
    pub global_functions: Vec<Method>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Class {
    #[serde(flatten)]
    pub common: BasicMember,
    pub methods: Vec<Method>,
    pub attributes: Vec<Attribute>,
    pub operators: Vec<Operator>,
    pub r#abstract: bool,
    pub parent: Option<String>,
    pub visibility: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Method {
    #[serde(flatten)]
    pub common: BasicMember,
    pub raises: Option<Vec<EventRaised>>,
    pub subclasses: Option<Vec<String>>,
    pub parameters: Vec<Parameter>,
    pub variant_parameter_groups: Option<Vec<ParameterGroup>>,
    pub variant_parameter_description: Option<String>,
    pub variadic_parameter: Option<VariadicParameter>,
    pub format: MethodFormat,
    pub return_values: Vec<ReturnValue>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct VariadicParameter {
    pub r#type: Option<Type>,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct MethodFormat {
    takes_table: bool,
    table_optional: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct EventRaised {
    #[serde(flatten)]
    pub common: BasicMember,
    pub timeframe: String,
    pub optional: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub order: i32,
    pub description: String,
    pub r#type: Type,
    pub optional: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ParameterGroup {
    pub name: String,
    pub order: i32,
    pub description: String,
    pub parameters: Vec<Parameter>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ReturnValue {
    pub order: i32,
    pub description: String,
    pub r#type: Type,
    pub optional: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum Type {
    Simple(String),
    Complex(Box<ComplexType>),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "complex_type", rename_all = "snake_case")]
pub enum ComplexType {
    Type {
        value: Type,
        description: String,
    },
    Builtin,
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
    #[serde(rename = "LuaCustomTable")]
    LuaCustomTable {
        key: Type,
        value: Type,
    },
    Function {
        parameters: Vec<Type>,
    },
    Literal {
        value: serde_json::Value,
        description: Option<String>,
    },
    #[serde(rename = "LuaLazyLoadedValue")]
    LuaLazyLoadedValue {
        value: Type,
    },
    #[serde(rename = "LuaStruct")]
    LuaStruct {
        attributes: Vec<Attribute>,
    },
    Table {
        parameters: Vec<Parameter>,
        variant_parameter_groups: Option<Vec<ParameterGroup>>,
        variant_parameter_description: Option<String>,
    },
    Tuple {
        values: Vec<Type>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Attribute {
    #[serde(flatten)]
    pub common: BasicMember,
    pub visibility: Option<Vec<String>>,
    pub raises: Option<Vec<EventRaised>>,
    pub subclasses: Option<Vec<String>>,
    #[serde(flatten)]
    pub types: AttributeTypes,
    pub optional: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AttributeTypes {
    pub read_type: Option<Type>,
    pub write_type: Option<Type>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum Operator {
    Method(Method),
    Attribute(Attribute),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Event {
    #[serde(flatten)]
    pub common: BasicMember,
    pub data: Vec<Parameter>,
    pub filter: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Define {
    #[serde(flatten)]
    pub common: BasicMember,
    pub values: Option<Vec<BasicMember>>,
    pub subkeys: Option<Vec<Self>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Concept {
    #[serde(flatten)]
    pub common: BasicMember,
    pub r#type: Type,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GlobalObject {
    pub name: String,
    pub order: i32,
    pub description: String,
    pub r#type: Type,
}

impl Class {
    pub fn to_embed(&self, data: &Data) -> serenity::CreateEmbed<'_> {
        let url = format!(
            "https://lua-api.factorio.com/latest/classes/{}.html",
            self.common.name
        );
        self.common
            .create_embed(data)
            .author(
                serenity::CreateEmbedAuthor::new("Class")
                    .url("https://lua-api.factorio.com/latest/classes.html"),
            )
            .url(url)
    }
}

impl Method {
    pub fn to_embed(&self, parent: &Class, data: &Data) -> serenity::CreateEmbed<'_> {
        let mut sorted_params = self.parameters.clone();
        sorted_params.sort_unstable_by_key(|par| par.order);
        let parameters_str = if self.format.takes_table {
            let parameters = sorted_params
                .into_iter()
                .map(|par| {
                    let optional = if par.optional { "?" } else { "" };
                    format!("{}{}=...", par.name, optional)
                })
                .collect::<Vec<String>>()
                .join(", ");
            format!(r"{{{parameters}}}")
        } else {
            let parameters = sorted_params
                .into_iter()
                .map(|par| {
                    let optional = if par.optional { "?" } else { "" };
                    format!("{}{}", par.name, optional)
                })
                .collect::<Vec<String>>()
                .join(", ");
            format!(r"({parameters})")
        };

        let return_values = self
            .return_values
            .clone()
            .into_iter()
            .map(|rv| {
                let optional = if rv.optional { "?" } else { "" };
                format!("{}{optional}", rv.r#type)
            })
            .collect::<Vec<String>>()
            .join(", ");

        let returns_str = if return_values.is_empty() {
            String::new()
        } else {
            format!("**→** `{return_values}`\n")
        };

        let url = format!(
            "https://lua-api.factorio.com/latest/classes/{}.html#{}",
            parent.common.name, self.common.name
        );
        let description = format!(
            "{}{}",
            returns_str,
            resolve_internal_links(data, &self.common.description)
        )
        .truncate_for_embed(4096);
        serenity::CreateEmbed::new()
            .title(
                format!(
                    "{}::{}{}",
                    parent.common.name, self.common.name, parameters_str
                )
                .truncate_for_embed(256),
            )
            .description(description)
            .color(serenity::Colour::GOLD)
            .url(url)
    }
}

impl Attribute {
    pub fn to_embed(&self, parent: &Class, data: &Data) -> serenity::CreateEmbed<'_> {
        let optional = if self.optional { "?" } else { "" };
        let url = format!(
            "https://lua-api.factorio.com/latest/classes/{}.html#{}",
            parent.common.name, self.common.name
        );
        let description = format!(
            "```{}{}```{}",
            self.types,
            optional,
            resolve_internal_links(data, &self.common.description)
        )
        .truncate_for_embed(4096);
        serenity::CreateEmbed::new()
            .title(format!("{}::{}", parent.common.name, self.common.name).truncate_for_embed(256))
            .description(description)
            .color(serenity::Colour::GOLD)
            .url(url)
    }
}

impl Event {
    pub fn to_embed(&self, data: &Data) -> serenity::CreateEmbed<'_> {
        let url = format!(
            "https://lua-api.factorio.com/latest/events.html#{}",
            self.common.name
        );
        self.common
            .create_embed(data)
            .author(
                serenity::CreateEmbedAuthor::new("Event")
                    .url("https://lua-api.factorio.com/latest/events.html"),
            )
            .url(url)
    }
}

impl Define {
    pub fn to_embed(&self, data: &Data) -> serenity::CreateEmbed<'_> {
        let url = format!(
            "https://lua-api.factorio.com/latest/defines.html#defines.{}",
            self.common.name
        );
        self.common
            .create_embed(data)
            .author(
                serenity::CreateEmbedAuthor::new("Define")
                    .url("https://lua-api.factorio.com/latest/defines.html"),
            )
            .url(url)
    }
}

impl Concept {
    pub fn to_embed(&self, data: &Data) -> serenity::CreateEmbed<'_> {
        let url = format!(
            "https://lua-api.factorio.com/latest/concepts.html#{}",
            self.common.name
        );
        self.common
            .create_embed(data)
            .author(
                serenity::CreateEmbedAuthor::new("Concept")
                    .url("https://lua-api.factorio.com/latest/concepts.html"),
            )
            .url(url)
    }
}

impl BasicMember {
    pub fn create_embed(&self, data: &Data) -> serenity::CreateEmbed<'_> {
        serenity::CreateEmbed::new()
            .title(&self.name)
            .description(resolve_internal_links(data, &self.description).truncate_for_embed(4096))
            .color(serenity::Colour::GOLD)
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
            Self::Builtin => write!(f, "builtin"),
            Self::Union { options, .. } => {
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
            Self::Dictionary { key, value } | Self::LuaCustomTable { key, value } => {
                write!(f, "dictionary[{key} → {value}]")
            }
            Self::Function { parameters } => {
                let fun_parameters = parameters
                    .iter()
                    .map(|t| format!("{t}"))
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(f, "function({fun_parameters})")
            }
            Self::Literal { value, .. } => match value {
                serde_json::Value::String(str) => write!(f, r#""{str}""#),
                serde_json::Value::Bool(bool) => write!(f, "{bool}"),
                serde_json::Value::Number(num) => write!(f, "{num}"),
                _ => write!(f, ""),
            },
            Self::LuaLazyLoadedValue { value } => write!(f, "LuaLazyLoadedValue({value})"),
            Self::LuaStruct { .. } => write!(f, "LuaStruct"),
            Self::Table { .. } => write!(f, "table"),
            Self::Tuple { .. } => write!(f, "tuple"),
        }
    }
}

impl fmt::Display for AttributeTypes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match (&self.read_type, &self.write_type) {
            (Some(read), Some(write)) if read == write => {
                write!(f, "[RW] :: {read}")
            }
            (Some(read), Some(write)) => {
                write!(f, "[R] :: {read}\n[W] :: {write}")
            }
            (Some(read), None) => {
                write!(f, "[R] :: {read}")
            }
            (None, Some(write)) => {
                write!(f, "[W] :: {write}")
            }
            (None, None) => write!(f, ""), // This case should never happen
        }
    }
}

pub async fn update_api_cache(cache: Arc<RwLock<ApiResponse>>) -> Result<(), Error> {
    {
        let new_runtime_api = get_runtime_api().await?;
        let mut c = match cache.write() {
            Ok(c) => c,
            Err(e) => return Err(ApiError::CacheError(e.to_string()))?,
        };
        *c = new_runtime_api;
    }
    Ok(())
}

pub async fn get_runtime_api() -> Result<ApiResponse, Error> {
    let url = "https://lua-api.factorio.com/latest/runtime-api.json";
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

pub fn generate_autocomplete_cache(
    api_cache: &Arc<RwLock<ApiResponse>>,
    autocomplete_cache: &Arc<RwLock<Vec<AutocompleteOption>>>,
) -> Result<(), Error> {
    let api = match api_cache.read() {
        Ok(c) => c,
        Err(e) => return Err(ApiError::CacheError(e.to_string()))?,
    }
    .clone();

    let runtime_autocomplete = api
        .classes
        .iter()
        .flat_map(|c| {
            std::iter::once(AutocompleteOption {
                kind: AutocompleteType::Class,
                search: c.common.name.clone(),
                parent: None,
            })
            .chain(c.methods.iter().map(|cm| AutocompleteOption {
                kind: AutocompleteType::Class,
                search: cm.common.name.clone(),
                parent: Some(c.common.name.clone()),
            }))
            .chain(c.attributes.iter().map(|ca| AutocompleteOption {
                kind: AutocompleteType::Class,
                search: ca.common.name.clone(),
                parent: Some(c.common.name.clone()),
            }))
        })
        .chain(api.concepts.iter().map(|c| AutocompleteOption {
            kind: AutocompleteType::Concept,
            search: c.common.name.clone(),
            parent: None,
        }))
        .chain(api.events.iter().map(|e| AutocompleteOption {
            kind: AutocompleteType::Event,
            search: e.common.name.clone(),
            parent: None,
        }))
        .collect::<Vec<AutocompleteOption>>();

    let mut c = match autocomplete_cache.write() {
        Ok(c) => c,
        Err(e) => return Err(ApiError::CacheError(e.to_string()))?,
    };
    *c = runtime_autocomplete;
    drop(c);
    Ok(())
}

pub async fn api_class(
    ctx: Context<'_>,
    class_search: &str,
    property_search: Option<&str>,
) -> Result<(), Error> {
    let cache = ctx.data().runtime_api_cache.clone();
    let api = match cache.read() {
        Ok(c) => c,
        Err(e) => return Err(ApiError::CacheError(e.to_string()))?,
    }
    .clone();

    // split_inputs(&mut class_search, &mut property_search);

    let Some(search_result) = api
        .classes
        .iter()
        .find(|class| class_search.eq_ignore_ascii_case(&class.common.name))
    else {
        return Err(ApiError::ClassNotFound(class_search.to_string()))?;
    };

    let embed = if let Some(property_name) = property_search {
        let method = search_result
            .methods
            .iter()
            .find(|m| m.common.name.eq_ignore_ascii_case(property_name));
        let attribute = search_result
            .attributes
            .iter()
            .find(|a| a.common.name.eq_ignore_ascii_case(property_name));

        if let Some(m) = method {
            m.to_embed(search_result, &ctx.data())
        } else if let Some(a) = attribute {
            a.to_embed(search_result, &ctx.data())
        } else {
            return Err(ApiError::PropertyNotFound(property_name.to_string()))?;
        }
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

pub async fn api_event(ctx: Context<'_>, event_search: &str) -> Result<(), Error> {
    let cache = ctx.data().runtime_api_cache.clone();
    let api = match cache.read() {
        Ok(c) => c,
        Err(e) => return Err(ApiError::CacheError(e.to_string()))?,
    }
    .clone();

    let Some(search_result) = api
        .events
        .iter()
        .find(|event| event_search.eq_ignore_ascii_case(&event.common.name))
    else {
        return Err(ApiError::EventNotFound(event_search.to_string()))?;
    };

    let builder = CreateReply::default()
        .embed(search_result.to_embed(&ctx.data()))
        .reply(true)
        .allowed_mentions(serenity::CreateAllowedMentions::default());
    ctx.send(builder).await?;
    Ok(())
}

/// Link a runtime modding API define
#[allow(clippy::unused_async)]
#[poise::command(
    slash_command,
    rename = "define",
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn api_define(
    ctx: Context<'_>,
    #[description = "Search term"]
    #[autocomplete = "autocomplete_define"]
    #[rename = "define"]
    define_search: String,
) -> Result<(), Error> {
    let cache = ctx.data().runtime_api_cache.clone();
    let api = match cache.read() {
        Ok(c) => c,
        Err(e) => return Err(ApiError::CacheError(e.to_string()))?,
    }
    .clone();

    let Some(search_result) = api
        .defines
        .iter()
        .find(|define| define_search.eq_ignore_ascii_case(&define.common.name))
    else {
        return Err(ApiError::DefineNotFound(define_search))?;
    };
    let builder = CreateReply::default()
        .embed(search_result.to_embed(&ctx.data()))
        .reply(true)
        .allowed_mentions(serenity::CreateAllowedMentions::default());
    ctx.send(builder).await?;
    Ok(())
}

#[allow(clippy::unused_async)]
async fn autocomplete_define<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> serenity::CreateAutocompleteResponse<'a> {
    let cache = ctx.data().runtime_api_cache.clone();
    let api = match cache.read() {
        Ok(c) => c,
        Err(e) => {
            error! {"Error acquiring cache: {e}"}
            return poise::serenity_prelude::CreateAutocompleteResponse::new();
        }
    }
    .clone();
    let choices = api
        .defines
        .iter()
        .filter(|c| {
            c.common
                .name
                .to_lowercase()
                .contains(partial.to_lowercase().trim())
        })
        .take(25)
        .map(|c| serenity::AutocompleteChoice::from(c.common.name.clone()))
        .collect::<Vec<serenity::AutocompleteChoice>>();

    serenity::CreateAutocompleteResponse::new().set_choices(choices)
}

pub async fn api_concept(ctx: Context<'_>, concept_search: &str) -> Result<(), Error> {
    let cache = ctx.data().runtime_api_cache.clone();
    let api = match cache.read() {
        Ok(c) => c,
        Err(e) => return Err(ApiError::CacheError(e.to_string()))?,
    }
    .clone();

    let Some(search_result) = api
        .concepts
        .iter()
        .find(|concept| concept_search.eq_ignore_ascii_case(&concept.common.name))
    else {
        return Err(ApiError::ConceptNotFound(concept_search.to_string()))?;
    };

    let builder = CreateReply::default()
        .embed(search_result.to_embed(&ctx.data()))
        .reply(true)
        .allowed_mentions(serenity::CreateAllowedMentions::default());
    ctx.send(builder).await?;
    Ok(())
}

/// Link to the runtime modding API
#[allow(clippy::unused_async)]
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn runtime(
    ctx: Context<'_>,
    #[description = "Search term"]
    #[autocomplete = "autocomplete_runtime"]
    search: String,
) -> Result<(), Error> {
    if search.starts_with("[Cls] ") {
        let searchtext = search.trim_start_matches("[Cls] ");
        if let Some((prototype, property)) = searchtext.split_once("::") {
            api_class(ctx, prototype, Some(property)).await?;
        } else {
            api_class(ctx, searchtext, None).await?;
        }
    } else if search.starts_with("[Con] ") {
        let property = search.trim_start_matches("[Con] ");
        api_concept(ctx, property).await?;
    } else if search.starts_with("[E] ") {
        let event = search.trim_start_matches("[E] ");
        api_event(ctx, event).await?;
    }
    Ok(())
}

#[derive(Debug, Clone)]
enum AutocompleteType {
    Class,
    Concept,
    Event,
}

#[derive(Debug, Clone)]
pub struct AutocompleteOption {
    kind: AutocompleteType,
    search: String,
    parent: Option<String>,
}

impl fmt::Display for AutocompleteOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            AutocompleteType::Class => write!(f, "[Cls] "),
            AutocompleteType::Concept => write!(f, "[Con] "),
            AutocompleteType::Event => write!(f, "[E] "),
        }?;
        if let Some(parent) = &self.parent {
            write!(f, "{}::{}", parent, self.search)
        } else {
            write!(f, "{}", self.search)
        }
    }
}

#[allow(clippy::unused_async)]
async fn autocomplete_runtime<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> serenity::CreateAutocompleteResponse<'a> {
    let cache = ctx.data().runtime_api_autocomplete_cache.clone();
    let runtime_autocomplete = match cache.read() {
        Ok(c) => c,
        Err(e) => {
            error! {"Error acquiring cache: {e}"}
            return serenity::CreateAutocompleteResponse::new();
        }
    }
    .clone();

    let choices = runtime_autocomplete
        .iter()
        .filter(|t| {
            t.search
                .to_lowercase()
                .contains(partial.to_lowercase().trim())
        })
        .take(25)
        .map(|t| serenity::AutocompleteChoice::from(format!("{t}")))
        .collect::<Vec<serenity::AutocompleteChoice>>();

    serenity::CreateAutocompleteResponse::new().set_choices(choices)
}

#[allow(unused_imports)]
mod tests {

    use super::*;
    use std::io::Read;

    #[tokio::test]
    async fn decode_api() {
        let file = std::fs::File::open("runtime-api-v6.json");
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
