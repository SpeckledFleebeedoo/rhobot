use serde::{Deserialize, Serialize};
use poise::serenity_prelude as serenity;
use poise::reply::CreateReply;
use std::{fmt, sync::{Arc, RwLock}};
use log::error;

use crate::{Context, Error, custom_errors::CustomError, api_data::api_data};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BasicMember {
    pub name: String,
    pub order: i32,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RuntimeApiResponse {
    pub application: String,
    pub application_version: String,
    pub api_version: i32,
    pub stage: String,
    pub classes: Vec<Class>,
    pub events: Vec<Event>,
    pub defines: Vec<Define>,
    pub builtin_types: Vec<BuiltinType>,
    pub concepts: Vec<Concept>,
    pub global_objects: Vec<GlobalObject>,
    pub global_functions: Vec<Method>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Class {
    #[serde(flatten)]
    pub common: BasicMember,
    pub notes: Option<Vec<String>>,
    pub examples: Option<Vec<String>>,
    pub methods: Vec<Method>,
    pub attributes: Vec<Attribute>,
    pub operators: Vec<Operator>,
    pub r#abstract: bool,
    pub base_classes: Option<Vec<String>>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Method {
    #[serde(flatten)]
    pub common: BasicMember,
    pub notes: Option<Vec<String>>,
    pub examples: Option<Vec<String>>,
    pub raises: Option<Vec<EventRaised>>,
    pub subclasses: Option<Vec<String>>,
    pub parameters: Vec<Parameter>,
    pub variant_parameter_groups: Option<Vec<ParameterGroup>>,
    pub variant_parameter_description: Option<String>,
    pub variadic_type: Option<Type>,
    pub variadic_description: Option<String>,
    pub takes_table: bool,
    pub table_is_optional: Option<bool>,
    pub return_values: Vec<ReturnValue>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EventRaised {
    #[serde(flatten)]
    pub common: BasicMember,
    pub timeframe: String,
    pub optional: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Parameter {
    #[serde(flatten)]
    pub common: BasicMember,
    pub r#type: Type,
    pub optional: bool,
}

// Does not use `common` as description needs to be optional for this type.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParameterGroup {
    pub name: String,
    pub order: i32,
    pub description: Option<String>,
    pub parameters: Vec<Parameter>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReturnValue {
    pub order: i32,
    pub description: String,
    pub r#type: Type,
    pub optional: bool,
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
    #[serde(rename = "LuaCustomTable")]
    LuaCustomTable { key: Type, value: Type },
    Function {parameters: Vec<Type>},
    Literal { value: serde_json::Value, description: Option<String> },
    #[serde(rename = "LuaLazyLoadedValue")]
    LuaLazyLoadedValue {value: Type},
    #[serde(rename = "LuaStruct")]
    LuaStruct {attributes: Vec<Attribute>},
    Table { parameters: Vec<Parameter> , variant_parameter_groups: Option<Vec<ParameterGroup>>, variant_parameter_description: Option<String> },
    Tuple { parameters: Vec<Parameter> , variant_parameter_groups: Option<Vec<ParameterGroup>>, variant_parameter_description: Option<String> },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Attribute {
    #[serde(flatten)]
    pub common: BasicMember,
    pub notes: Option<Vec<String>>,
    pub examples: Option<Vec<String>>,
    pub raises: Option<Vec<EventRaised>>,
    pub subclasses: Option<Vec<String>>,
    pub r#type: Type,
    pub optional: bool,
    pub read: bool,
    pub write: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Operator {
    Method(Method),
    Attribute(Attribute)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Event {
    #[serde(flatten)]
    pub common: BasicMember,
    pub notes: Option<Vec<String>>,
    pub examples: Option<Vec<String>>,
    pub data: Vec<Parameter>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Define {
    #[serde(flatten)]
    pub common: BasicMember,
    pub values: Option<Vec<BasicMember>>,
    pub subkeys: Option<Vec<Define>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BuiltinType {
    #[serde(flatten)]
    pub common: BasicMember,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Concept {
    #[serde(flatten)]
    pub common: BasicMember,
    pub notes: Option<Vec<String>>,
    pub examples: Option<Vec<String>>,
    pub r#type: Type,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GlobalObject {
    #[serde(flatten)]
    pub common: BasicMember,
    pub r#type: Type,
}

impl Class {
    pub fn to_embed(&self) -> serenity::CreateEmbed {
        let url = format!("https://lua-api.factorio.com/latest/classes/{}.html", &self.common.name);
        self.common.create_embed()
        .author(serenity::CreateEmbedAuthor::new("Class")
            .url("https://lua-api.factorio.com/latest/classes.html"))
        .url(url)
    }
}

impl Event {
    pub fn to_embed(&self) -> serenity::CreateEmbed {
        let url = format!("https://lua-api.factorio.com/latest/events.html#{}", &self.common.name);
        self.common.create_embed()
        .author(serenity::CreateEmbedAuthor::new("Event")
            .url("https://lua-api.factorio.com/latest/events.html"))
        .url(url)
    }
}

impl Define {
    pub fn to_embed(&self) -> serenity::CreateEmbed {
        let url = format!("https://lua-api.factorio.com/latest/defines.html#defines.{}", &self.common.name);
        self.common.create_embed()
        .author(serenity::CreateEmbedAuthor::new("Define")
            .url("https://lua-api.factorio.com/latest/defines.html"))
        .url(url)
    }
}

impl Concept {
    pub fn to_embed(&self) -> serenity::CreateEmbed {
        let url = format!("https://lua-api.factorio.com/latest/concepts.html#{}", &self.common.name);
        self.common.create_embed()
        .author(serenity::CreateEmbedAuthor::new("Concept")
            .url("https://lua-api.factorio.com/latest/concepts.html"))
        .url(url)
    }
}

impl BuiltinType {
    pub fn to_embed(&self) -> serenity::CreateEmbed {
        let url = format!("https://lua-api.factorio.com/latest/builtin-types.html#{}", &self.common.name);
        self.common.create_embed()
        .author(serenity::CreateEmbedAuthor::new("Builtin type")
            .url("https://lua-api.factorio.com/latest/builtin-types.html"))
        .url(url)
    }
}

impl BasicMember {
    pub fn create_embed(&self) -> serenity::CreateEmbed {
        serenity::CreateEmbed::new()
            .title(&self.name)
            .description(&self.description)
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
            Self::Type { value, .. } => {write!(f, "{value}")},
            Self::Union { options, .. } => {
                let options_string = options.iter()
                    .map(|t| format!("{t}"))
                    .collect::<Vec<String>>()
                    .join(" or ");
                write!(f, "{options_string}")
            },
            Self::Array { value } => {write!(f, "array[{value}]")},
            Self::Dictionary { key, value } | Self::LuaCustomTable { key, value } => {
                write!(f, "dictionary[{key} 🡪 {value}]")
            },
            Self::Function { parameters } => {
                let fun_parameters = parameters.iter()
                    .map(|t| format!("{t}"))
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(f, "function({fun_parameters})")
            },
            Self::Literal { value, .. } => {
                match value {
                    serde_json::Value::String(str) => write!(f, r#""{}""#, &str),
                    serde_json::Value::Bool(bool) => write!(f, "{bool}"),
                    serde_json::Value::Number(num) => write!(f, "{num}"),
                    _ => write!(f, ""),
                }
            },
            Self::LuaLazyLoadedValue { value } => write!(f, "LuaLazyLoadedValue({value})"),
            Self::LuaStruct { .. } => write!(f, "LuaStruct"),
            Self::Table { .. } => write!(f, "table"),
            Self::Tuple { .. } => write!(f, "tuple"),
        }
    }
}

pub async fn update_api_cache(
    cache: Arc<RwLock<RuntimeApiResponse>>,
) -> Result<(), Error> {
    println!("Updating API cache");
    {
    let new_runtime_api = get_runtime_api().await?;
    let mut c = match cache.write() {
        Ok(c) => c,
        Err(e) => {
            return Err(Box::new(CustomError::new(&format!("Error acquiring cache: {e}"))));
        },
    };
    *c = new_runtime_api;
    }
    Ok(())
}

pub async fn get_runtime_api() -> Result<RuntimeApiResponse, Error> {
    let response = reqwest::get("https://lua-api.factorio.com/latest/runtime-api.json").await?;

    match response.status() {
        reqwest::StatusCode::OK => (),
        _ => return Err(Box::new(CustomError::new("Received HTTP status code that is not 200")))
    };
    Ok(response.json::<RuntimeApiResponse>().await?)
}

/// Link a page in the mod making API. Slash commands only.
#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, track_edits, subcommands("api_runtime", "api_data"))]
pub async fn api(
    _ctx: Context<'_>
) -> Result<(), Error> {
    Ok(())
}

#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, track_edits, subcommands("api_class", "api_event", "api_define", "api_concept", "api_builtintype"), rename="runtime")]
pub async fn api_runtime(
    _ctx: Context<'_>
) -> Result<(), Error> {
    Ok(())
}

#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, track_edits, rename="class")]
pub async fn api_class (
    ctx: Context<'_>,
    #[description = "Search term"]
    #[autocomplete = "autocomplete_class"]
    #[rename = "class"]
    class_search: String,
    #[description = "Class property"]
    #[autocomplete = "autocomplete_class_property"]
    #[rename = "property"]
    property_search: Option<String>,
) -> Result<(), Error> {

    let cache = ctx.data().runtime_api_cache.clone();
    let api = match cache.read() {
        Ok(c) => c,
        Err(e) => {
            return Err(Box::new(CustomError::new(&format!("Error acquiring cache: {e}"))));
        },
    }.clone();
    let Some(search_result) = api.classes.iter()
        .find(|class| class_search.eq_ignore_ascii_case(&class.common.name)) 
    else {
        return Err(Box::new(CustomError::new("Could not find specified class in runtime API documentation")));
    };

    let mut embed = search_result.to_embed();
    if let Some(property_name) = property_search {
        let method = search_result.methods.clone()
            .into_iter()
            .find(|m| m.common.name == property_name);
        let attribute = search_result.attributes.clone()
            .into_iter()
            .find(|a| a.common.name == property_name);

        if let Some(m) = method {
            let parameters_str = if m.takes_table {
                    let mut sorted_params = m.parameters.clone();
                    sorted_params.sort_unstable_by_key(|par| par.common.order);
                    let parameters = sorted_params.into_iter().map(|par| {
                        let optional = if par.optional { "?" } else { "" };
                        format!("{}{}=...", par.common.name, optional)
                    }).collect::<Vec<String>>().join(", ");
                    format!(r#"{{{parameters}}}"#)
                } else {
                    let mut sorted_params = m.parameters.clone();
                    sorted_params.sort_unstable_by_key(|par| par.common.order);
                    let parameters = sorted_params.into_iter().map(|par| {
                        let optional = if par.optional { "?" } else { "" };
                        format!("{}{}", par.common.name, optional)
                    }).collect::<Vec<String>>().join(", ");
                    format!(r#"({parameters})"#)
            };
            
            let return_values = m.return_values.into_iter().map(|rv| {
                let optional = if rv.optional { "?" } else { "" };
                format!("{}{optional}", rv.r#type)
            }
            ).collect::<Vec<String>>().join(", ");
            embed = embed.field(format!("`{}{} 🡪 {}`", m.common.name, parameters_str, return_values), m.common.description, false);

        } else if let Some(a) = attribute {
            let rw = match (a.read, a.write) {
                (true, true) => "[RW]",
                (true, false) => "[R]",
                (false, true) => "[W]",
                (false, false) => ""
            };
            let optional = if a.optional { "?" } else { "" };
            embed = embed.field(format!(
                "`{} {} :: {}{}`", a.common.name, rw, a.r#type, optional), 
                a.common.description, 
                false
            );
        };        
    };
    let builder = CreateReply::default()
        .embed(embed);
    ctx.send(builder).await?;
    Ok(())
}

#[allow(clippy::unused_async)]
async fn autocomplete_class<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let cache = ctx.data().runtime_api_cache.clone();
    let api = match cache.read(){
        Ok(c) => c,
        Err(e) => {
            error!{"Error acquiring cache: {e}"}
            return vec![]
        },
    }.clone();
    api.classes.iter()
        .filter(|c| c.common.name.to_lowercase().starts_with(&partial.to_lowercase()))
        .map(|c| c.common.name.clone())
        .collect::<Vec<String>>()
}

#[allow(clippy::unused_async)]
async fn autocomplete_class_property<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let mut classname = String::new();
    if let poise::Context::Application(appcontext) = ctx {
        if let serenity::ResolvedValue::String(st) = appcontext.args[0].value {
            st.clone_into(&mut classname);
        }
    }

    if classname.is_empty() {
        return vec![];   // Happens when property field is used before class field
    }
    let cache = ctx.data().runtime_api_cache.clone();
    let api = match cache.read(){
        Ok(c) => c,
        Err(e) => {
            error!{"Error acquiring cache: {e}"}
            return vec![]
        },
    }.clone();
    let Some(class) = api.classes.iter()
        .find(|c| c.common.name == classname)
    else {return vec![]};    // Happens when invalid class is used
    
    let methods = class.methods.clone().into_iter().map(|m| m.common);
    let attributes = class.attributes.clone().into_iter().map(|a| a.common);
    let properties = methods.chain(attributes);
    
    properties.map(|p| p.name)
        .filter(|n| n.to_lowercase().contains(&partial.to_lowercase()))
        .collect::<Vec<String>>()
}

#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, track_edits, rename="event")]
pub async fn api_event (
    ctx: Context<'_>,
    #[description = "Search term"]
    #[autocomplete = "autocomplete_event"]
    #[rename = "event"]
    event_search: String,
) -> Result<(), Error> {

    let cache = ctx.data().runtime_api_cache.clone();
    let api = match cache.read() {
        Ok(c) => c,
        Err(e) => {
            return Err(Box::new(CustomError::new(&format!("Error acquiring cache: {e}"))));
        },
    }.clone();

    let Some(search_result) = api.events.iter()
        .find(|event| event_search.eq_ignore_ascii_case(&event.common.name)) 
        else {
            return Err(Box::new(CustomError::new("Could not find specified event in runtime API documentation")));
        };

    let builder = CreateReply::default()
        .embed(search_result.to_embed());
    ctx.send(builder).await?;
    Ok(())
}

#[allow(clippy::unused_async)]
async fn autocomplete_event<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let cache = ctx.data().runtime_api_cache.clone();
    let api = match cache.read(){
        Ok(c) => c,
        Err(e) => {
            error!{"Error acquiring cache: {e}"}
            return vec![]
        },
    }.clone();
    api.events.iter()
        .filter(|c| c.common.name.to_lowercase().starts_with(&partial.to_lowercase()))
        .map(|c| c.common.name.clone())
        .collect::<Vec<String>>()
}

#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, track_edits, rename="define")]
pub async fn api_define (
    ctx: Context<'_>,
    #[description = "Search term"]
    #[autocomplete = "autocomplete_define"]
    #[rename = "define"]
    define_search: String,
) -> Result<(), Error> {

    let cache = ctx.data().runtime_api_cache.clone();
    let api = match cache.read() {
        Ok(c) => c,
        Err(e) => {
            return Err(Box::new(CustomError::new(&format!("Error acquiring cache: {e}"))));
        },
    }.clone();

    let Some(search_result) = api.defines.iter()
        .find(|define| define_search.eq_ignore_ascii_case(&define.common.name)) 
    else {
        return Err(Box::new(CustomError::new("Could not find specified define type in runtime API documentation")));
    };
    let builder = CreateReply::default()
        .embed(search_result.to_embed());
    ctx.send(builder).await?;
    Ok(())
}

#[allow(clippy::unused_async)]
async fn autocomplete_define<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let cache = ctx.data().runtime_api_cache.clone();
    let api = match cache.read(){
        Ok(c) => c,
        Err(e) => {
            error!{"Error acquiring cache: {e}"}
            return vec![]
        },
    }.clone();
    api.defines.iter()
        .filter(|c| c.common.name.to_lowercase().starts_with(&partial.to_lowercase()))
        .map(|c| c.common.name.clone())
        .collect::<Vec<String>>()
}

#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, track_edits, rename="concept")]
pub async fn api_concept (
    ctx: Context<'_>,
    #[description = "Search term"]
    #[autocomplete = "autocomplete_concept"]
    #[rename = "concept"]
    concept_search: String,
) -> Result<(), Error> {

    let cache = ctx.data().runtime_api_cache.clone();
    let api = match cache.read() {
        Ok(c) => c,
        Err(e) => {
            return Err(Box::new(CustomError::new(&format!("Error acquiring cache: {e}"))));
        },
    }.clone();

    let Some(search_result) = api.concepts.iter()
        .find(|concept| concept_search.eq_ignore_ascii_case(&concept.common.name)) 
    else {
        return Err(Box::new(CustomError::new("Could not find specified concept type in runtime API documentation")))
    };

    let builder = CreateReply::default()
        .embed(search_result.to_embed());
    ctx.send(builder).await?;
    Ok(())
}

#[allow(clippy::unused_async)]
async fn autocomplete_concept<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let cache = ctx.data().runtime_api_cache.clone();
    let api = match cache.read(){
        Ok(c) => c,
        Err(e) => {
            error!{"Error acquiring cache: {e}"}
            return vec![]
        },
    }.clone();
    api.concepts.iter()
        .filter(|c| c.common.name.to_lowercase().starts_with(&partial.to_lowercase()))
        .map(|c| c.common.name.clone())
        .collect::<Vec<String>>()
}

#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, track_edits, rename="builtin_type")]
pub async fn api_builtintype (
    ctx: Context<'_>,
    #[description = "Search term"]
    #[autocomplete = "autocomplete_builtintype"]
    #[rename = "builtin_type"]
    builtintype_search: String,
) -> Result<(), Error> {

    let cache = ctx.data().runtime_api_cache.clone();
    let api = match cache.read() {
        Ok(c) => c,
        Err(e) => {
            return Err(Box::new(CustomError::new(&format!("Error acquiring cache: {e}"))));
        },
    }.clone();

    let Some(search_result) = api.builtin_types.iter()
        .find(|builtin_type| builtintype_search.eq_ignore_ascii_case(&builtin_type.common.name)) 
    else {
        return Err(Box::new(CustomError::new("Could not find specified builtin type in runtime API documentation")))
    };
    let builder = CreateReply::default()
        .embed(search_result.to_embed());
    ctx.send(builder).await?;
    Ok(())
}

#[allow(clippy::unused_async)]
async fn autocomplete_builtintype<'a> (
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let cache = ctx.data().runtime_api_cache.clone();
    let api = match cache.read(){
        Ok(c) => c,
        Err(e) => {
            error!{"Error acquiring cache: {e}"}
            return vec![]
        },
    }.clone();
    api.builtin_types.iter()
        .filter(|c| c.common.name.to_lowercase().starts_with(&partial.to_lowercase()))
        .map(|c| c.common.name.clone())
        .collect::<Vec<String>>()
}