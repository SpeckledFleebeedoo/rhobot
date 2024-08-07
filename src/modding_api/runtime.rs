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
    formatting_tools::DiscordFormat, 
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
pub struct Image {
    pub filename: String,
    caption: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
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


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct VariadicParameter {
    pub r#type: Option<Type>,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MethodFormat {
    takes_table: bool,
    table_optional: Option<bool>,
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
    pub name: String,
    pub order: i32,
    pub description: String,
    pub r#type: Type,
    pub optional: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ParameterGroup {
    pub name: String,
    pub order: i32,
    pub description: String,
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
    Builtin,
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
    Tuple { values: Vec<Type> },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Attribute {
    #[serde(flatten)]
    pub common: BasicMember,
    pub visibility: Option<Vec<String>>,
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
    pub data: Vec<Parameter>,
    pub filter: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Define {
    #[serde(flatten)]
    pub common: BasicMember,
    pub values: Option<Vec<BasicMember>>,
    pub subkeys: Option<Vec<Define>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Concept {
    #[serde(flatten)]
    pub common: BasicMember,
    pub r#type: Type,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GlobalObject {
    pub name: String,
    pub order: i32,
    pub description: String,
    pub r#type: Type,
}

impl Class {
    pub fn to_embed(&self, data: &Data) -> serenity::CreateEmbed {
        let url = format!("https://lua-api.factorio.com/latest/classes/{}.html", &self.common.name);
        self.common.create_embed(data)
        .author(serenity::CreateEmbedAuthor::new("Class")
            .url("https://lua-api.factorio.com/latest/classes.html"))
        .url(url)
    }
}

impl Event {
    pub fn to_embed(&self, data: &Data) -> serenity::CreateEmbed {
        let url = format!("https://lua-api.factorio.com/latest/events.html#{}", &self.common.name);
        self.common.create_embed(data)
        .author(serenity::CreateEmbedAuthor::new("Event")
            .url("https://lua-api.factorio.com/latest/events.html"))
        .url(url)
    }
}

impl Define {
    pub fn to_embed(&self, data: &Data) -> serenity::CreateEmbed {
        let url = format!("https://lua-api.factorio.com/latest/defines.html#defines.{}", &self.common.name);
        self.common.create_embed(data)
        .author(serenity::CreateEmbedAuthor::new("Define")
            .url("https://lua-api.factorio.com/latest/defines.html"))
        .url(url)
    }
}

impl Concept {
    pub fn to_embed(&self, data: &Data) -> serenity::CreateEmbed {
        let url = format!("https://lua-api.factorio.com/latest/concepts.html#{}", &self.common.name);
        self.common.create_embed(data)
        .author(serenity::CreateEmbedAuthor::new("Concept")
            .url("https://lua-api.factorio.com/latest/concepts.html"))
        .url(url)
    }
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
            Self::Builtin => write!(f, "builtin"),
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
    cache: Arc<RwLock<ApiResponse>>,
) -> Result<(), Error> {
    info!("Updating API cache");
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

pub async fn get_runtime_api() -> Result<ApiResponse, Error> {
    let response = reqwest::get("https://lua-api.factorio.com/latest/runtime-api.json").await?;

    match response.status() {
        reqwest::StatusCode::OK => (),
        _ => return Err(Box::new(CustomError::new(&format!("Received HTTP status code {} while accessing Lua runtime API", response.status().as_str()))))
    };
    Ok(response.json::<ApiResponse>().await?)
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
        return Err(Box::new(CustomError::new(&format!("Could not find class `{class_search}` in runtime API documentation"))));
    };

    let mut embed = search_result.to_embed(ctx.data());
    if let Some(property_name) = property_search {
        let method = search_result.methods.clone()
            .into_iter()
            .find(|m| m.common.name.eq_ignore_ascii_case(&property_name));
        let attribute = search_result.attributes.clone()
            .into_iter()
            .find(|a| a.common.name.eq_ignore_ascii_case(&property_name));

        let c_name = &search_result.common.name;
        if let Some(m) = method {
            let parameters_str = if m.format.takes_table {
                    let mut sorted_params = m.parameters.clone();
                    sorted_params.sort_unstable_by_key(|par| par.order);
                    let parameters = sorted_params.into_iter().map(|par| {
                        let optional = if par.optional { "?" } else { "" };
                        format!("{}{}=...", par.name, optional)
                    }).collect::<Vec<String>>().join(", ");
                    format!(r#"{{{parameters}}}"#)
                } else {
                    let mut sorted_params = m.parameters.clone();
                    sorted_params.sort_unstable_by_key(|par| par.order);
                    let parameters = sorted_params.into_iter().map(|par| {
                        let optional = if par.optional { "?" } else { "" };
                        format!("{}{}", par.name, optional)
                    }).collect::<Vec<String>>().join(", ");
                    format!(r#"({parameters})"#)
            };
            
            let return_values = m.return_values.into_iter().map(|rv| {
                let optional = if rv.optional { "?" } else { "" };
                format!("{}{optional}", rv.r#type)
            }
            ).collect::<Vec<String>>().join(", ");
            let name = &m.common.name;
            let full_docs_link = format!("\n[Full documentation](https://lua-api.factorio.com/latest/classes/{c_name}.html#{name})");
            let description = resolve_internal_links(ctx.data(), &m.common.description).
                truncate_for_embed(1024 - full_docs_link.len());
            let title = if return_values.is_empty() {
                format!("`{name}{}`", parameters_str.truncate_for_embed(256 - name.len() - 2))
            } else {
                format!("`{name}{} 🡪 {return_values}`", parameters_str.truncate_for_embed(256 - name.len() - return_values.len() - 5))
            };
            embed = embed.field(
                title, 
                format!("{description}{full_docs_link}"), 
                false
            );

        } else if let Some(a) = attribute {
            let rw = match (a.read, a.write) {
                (true, true) => "[RW]",
                (true, false) => "[R]",
                (false, true) => "[W]",
                (false, false) => ""
            };
            let name = &a.common.name;
            let a_type = &a.r#type;
            let optional = if a.optional { "?" } else { "" };
            let full_docs_link = format!("\n[Full documentation](https://lua-api.factorio.com/latest/classes/{c_name}.html#{name})");
            let description = resolve_internal_links(ctx.data(), &a.common.description)
                .truncate_for_embed(1024 - full_docs_link.len());
            embed = embed.field(
                format!("`{name} {rw} :: {a_type}{optional}`").truncate_for_embed(256), 
                format!("{description}{full_docs_link}"), 
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
    let poise::Context::Application(appcontext) = ctx else {return vec![]};
    let serenity::ResolvedValue::String(classname) = appcontext.args[0].value else {return vec![]};
    if classname.is_empty() {
        return vec![];
    };

    let cache = ctx.data().runtime_api_cache.clone();
    let api = match cache.read(){
        Ok(c) => c,
        Err(e) => {
            error!{"Error acquiring cache: {e}"}
            return vec![]
        },
    }.clone();
    let Some(class) = api.classes.iter()
        .find(|c| c.common.name.eq_ignore_ascii_case(classname))
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
            return Err(Box::new(CustomError::new(&format!("Could not find event `{event_search}` in runtime API documentation"))));
        };

    let builder = CreateReply::default()
        .embed(search_result.to_embed(ctx.data()));
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
        return Err(Box::new(CustomError::new(&format!("Could not find define `{define_search}` in runtime API documentation"))));
    };
    let builder = CreateReply::default()
        .embed(search_result.to_embed(ctx.data()));
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
        return Err(Box::new(CustomError::new(&format!("Could not find concept `{concept_search}` in runtime API documentation"))))
    };

    let builder = CreateReply::default()
        .embed(search_result.to_embed(ctx.data()));
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

#[allow(unused_imports)]
mod tests {

    use super::*;
    use std::io::Read;
    
    #[tokio::test]
    async fn decode_api() {
        let file = std::fs::File::open("runtime-api-v5.json");
        assert!(file.is_ok(), "Failed to read file");

        let buf_reader = std::io::BufReader::new(file.unwrap());
        let api_data: Result<ApiResponse, serde_json::Error> = serde_json::from_reader(buf_reader);
        match api_data {
            Ok(_) => {},
            Err(e) => {panic!("{}", e)}
        };
    }
}
