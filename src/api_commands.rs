use serde::{Deserialize, Serialize};
use poise::serenity_prelude as serenity;
use poise::reply::CreateReply;
use std::{borrow::Borrow, sync::{Arc, RwLock}};

use crate::{Context, Error, custom_errors::CustomError};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BasicMember {
    pub name: String,
    pub order: i32,
    pub description: String,
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
#[serde(tag = "complex_type")]
pub enum ComplexType {
    #[serde(rename = "type")]
    Type {value: Type, description: String },
    #[serde(rename = "union")]
    Union { options: Vec<Type>, full_format: bool },
    #[serde(rename = "array")]
    Array { value: Type },
    #[serde(rename = "dictionary")]
    Dictionary { key: Type, value: Type },
    LuaCustomTable { key: Type, value: Type },
    #[serde(rename = "function")]
    Function {parameters: Vec<Type>},
    #[serde(rename = "literal")]
    Literal { value: serde_json::Value, description: Option<String> },
    LuaLazyLoadedValue {value: Type},
    LuaStruct {attributes: Vec<Attribute>},
    #[serde(rename = "table")]
    Table { parameters: Vec<Parameter> , variant_parameter_groups: Option<Vec<ParameterGroup>>, variant_parameter_description: Option<String> },
    #[serde(rename = "tuple")]
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
    pub async fn to_embed(&self) -> serenity::CreateEmbed {
        let url = format!("https://lua-api.factorio.com/latest/classes/{}.html", &self.common.name);
        
        self.common.create_embed(&url).await
    }
}

impl Event {
    pub async fn to_embed(&self) -> serenity::CreateEmbed {
        let url = format!("https://lua-api.factorio.com/latest/events.html#{}", &self.common.name);
        
        self.common.create_embed(&url).await
    }
}

impl Define {
    pub async fn to_embed(&self) -> serenity::CreateEmbed {
        let url = format!("https://lua-api.factorio.com/latest/defines.html#defines.{}", &self.common.name);
        
        self.common.create_embed(&url).await
    }
}

impl Concept {
    pub async fn to_embed(&self) -> serenity::CreateEmbed {
        let url = format!("https://lua-api.factorio.com/latest/concepts.html#{}", &self.common.name);
        
        self.common.create_embed(&url).await
    }
}

impl BuiltinType {
    pub async fn to_embed(&self) -> serenity::CreateEmbed {
        let url = format!("https://lua-api.factorio.com/latest/builtin-types.html#{}", &self.common.name);
        
        self.common.create_embed(&url).await
    }
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

impl Type {
    pub fn to_str(&self) -> String {
        let mut output = String::new();
        match self {
            Type::Simple(str) => output.push_str(str),
            Type::Complex(ct) => {
                match ct.borrow() {
                    ComplexType::Type { value, .. } => {
                        output.push_str(&value.to_str());
                    },
                    ComplexType::Union { options, .. } => {
                        let options_string = options.iter()
                            .map(|opt| opt.to_str())
                            .collect::<Vec<String>>()
                            .join(" or ");
                        output.push_str(&options_string)
                    },
                    ComplexType::Array { value } => {
                        output.push_str(&format!("array[{}]", &value.to_str()));
                    }
                    ComplexType::Dictionary{ key, value } | ComplexType::LuaCustomTable{ key, value } => {
                        output.push_str(&format!("dictionary[{} 🡪 {}]", &key.to_str(), &value.to_str()));
                    }
                    ComplexType::Function { parameters } => {
                        let fun_parameters = parameters.iter()
                            .map(|param| param.to_str())
                            .collect::<Vec<String>>()
                            .join(", ");
                        output.push_str(&format!("function({})", fun_parameters));
                    }
                    ComplexType::Literal { value, .. } => {
                        match value {
                            serde_json::Value::String(str) => output.push_str(&format!(r#""{}""#, &str)),
                            serde_json::Value::Bool(bool) => output.push_str(&bool.to_string()),
                            serde_json::Value::Number(num) => output.push_str(&num.to_string()),
                            _ => ()
                        }
                    }
                    ComplexType::LuaLazyLoadedValue { value } => {
                        output.push_str(&format!("LuaLazyLoadedValue({})", &value.to_str()));
                    }
                    ComplexType::LuaStruct { .. } => {
                        output.push_str("LuaStruct");
                    }
                    ComplexType::Table { .. } => {
                        output.push_str("table");
                    }
                    ComplexType::Tuple { .. } => {
                        output.push_str("tuple");
                    }
                }
            }
        }
        output
    }
}

pub async fn update_api_cache(
    cache: Arc<RwLock<ApiResponse>>,
) -> Result<(), Error> {
    let new_api = get_runtime_api().await?;
    let mut c = cache.write().unwrap();
    *c = new_api;
    Ok(())
}

pub async fn get_runtime_api() -> Result<ApiResponse, Error> {
    let response = reqwest::get("https://lua-api.factorio.com/latest/runtime-api.json").await?;

    match response.status() {
        reqwest::StatusCode::OK => (),
        _ => return Err(Box::new(CustomError::new("Received HTTP status code that is not 200")))
    };
    Ok(response.json::<ApiResponse>().await?)
}

#[poise::command(prefix_command, slash_command, guild_only, subcommands("api_runtime"))]
pub async fn api(
    _ctx: Context<'_>
) -> Result<(), Error> {
    Ok(())
}

#[poise::command(prefix_command, slash_command, guild_only, subcommands("api_class", "api_event", "api_define", "api_concept", "api_builtintype"), rename="runtime")]
pub async fn api_runtime(
    _ctx: Context<'_>
) -> Result<(), Error> {
    Ok(())
}

#[poise::command(prefix_command, slash_command, rename="class")]
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

    let cache = ctx.data().apicache.clone();
    let api = cache.read().unwrap().clone();
    let search_result = api.classes.iter()
        .find(|class| class_search.eq_ignore_ascii_case(&class.common.name)).unwrap();
    let mut embed = search_result.to_embed().await;
    if property_search.is_some() {
        let property_name = property_search.unwrap();
        let method = search_result.methods.clone()
            .into_iter()
            .find(|m| m.common.name == property_name);
        let attribute = search_result.attributes.clone()
            .into_iter()
            .find(|a| a.common.name == property_name);

        if let Some(m) = method {
            let parameters_str = match m.takes_table {
                true => {
                    let mut sorted_params = m.parameters.clone();
                    sorted_params.sort_unstable_by_key(|par| par.common.order);
                    let parameters = sorted_params.into_iter().map(|par| {
                        let optional = match par.optional {
                            true => "?",
                            false => "",
                        };
                        format!("{}{}=...", par.common.name, optional)
                    }).collect::<Vec<String>>().join(", ");
                    format!(r#"{{{}}}"#, parameters)
                },
                false => {
                    let mut sorted_params = m.parameters.clone();
                    sorted_params.sort_unstable_by_key(|par| par.common.order);
                    let parameters = sorted_params.into_iter().map(|par| {
                        let optional = match par.optional {
                            true => "?",
                            false => "",
                        };
                        format!("{}{}", par.common.name, optional)
                    }).collect::<Vec<String>>().join(", ");
                    format!(r#"({})"#, parameters)
                },
            };
            
            let return_values = m.return_values.into_iter().map(|rv| {
                let optional = match rv.optional {
                    true => "?",
                    false => "",
                };
                rv.r#type.to_str() + optional
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
            let optional = match a.optional {
                true => "?",
                false => "",
            };
            embed = embed.field(format!(
                "`{} {} :: {}{}`", a.common.name, rw, a.r#type.to_str(), optional), 
                a.common.description, 
                false
            );
        }

        
    };
    let builder = CreateReply::default()
        .embed(embed);
    ctx.send(builder).await?;
    Ok(())
}

async fn autocomplete_class<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let cache = ctx.data().apicache.clone();
    let api = cache.read().unwrap().clone();
    api.classes.iter()
        .filter(|c| c.common.name.to_lowercase().starts_with(&partial.to_lowercase()))
        .map(|c| c.common.name.clone())
        .collect::<Vec<String>>()
}

async fn autocomplete_class_property<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let mut classname = String::new();
    if let poise::Context::Application(appcontext) = ctx {
        if let serenity::ResolvedValue::String(st) = appcontext.args[0].value {
            classname = st.to_owned();
        }
    }

    if classname.is_empty() {
        return vec![];   // Should never happen
    }
    let cache = ctx.data().apicache.clone();
    let api = cache.read().unwrap().clone();
    let class = match api.classes.iter()
        .find(|c| c.common.name == classname) {
            Some(c) => c,
            None => {return vec![]},    // Only happens when autocomplete is not used
        };
    
    let methods = class.methods.clone().into_iter().map(|m| m.common);
    let attributes = class.attributes.clone().into_iter().map(|a| a.common);
    let properties = methods.chain(attributes);
    
    properties.map(|p| p.name)
        .filter(|n| n.to_lowercase().contains(&partial.to_lowercase()))
        .collect::<Vec<String>>()
}

#[poise::command(prefix_command, slash_command, rename="event")]
pub async fn api_event (
    ctx: Context<'_>,
    #[description = "Search term"]
    #[autocomplete = "autocomplete_event"]
    #[rename = "event"]
    event_search: String,
) -> Result<(), Error> {

    let cache = ctx.data().apicache.clone();
    let api = cache.read().unwrap().clone();

    let search_result = api.events.iter().find(|event| event_search.eq_ignore_ascii_case(&event.common.name)).unwrap();
    let builder = CreateReply::default()
        .embed(search_result.to_embed().await);
    ctx.send(builder).await?;
    Ok(())
}

async fn autocomplete_event<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let cache = ctx.data().apicache.clone();
    let api = cache.read().unwrap().clone();
    api.events.iter()
        .filter(|c| c.common.name.to_lowercase().starts_with(&partial.to_lowercase()))
        .map(|c| c.common.name.clone())
        .collect::<Vec<String>>()
}

#[poise::command(prefix_command, slash_command, rename="define")]
pub async fn api_define (
    ctx: Context<'_>,
    #[description = "Search term"]
    #[autocomplete = "autocomplete_define"]
    #[rename = "define"]
    define_search: String,
) -> Result<(), Error> {

    let cache = ctx.data().apicache.clone();
    let api = cache.read().unwrap().clone();

    let search_result = api.defines.iter().find(|define| define_search.eq_ignore_ascii_case(&define.common.name)).unwrap();
    let builder = CreateReply::default()
        .embed(search_result.to_embed().await);
    ctx.send(builder).await?;
    Ok(())
}

async fn autocomplete_define<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let cache = ctx.data().apicache.clone();
    let api = cache.read().unwrap().clone();
    api.defines.iter()
        .filter(|c| c.common.name.to_lowercase().starts_with(&partial.to_lowercase()))
        .map(|c| c.common.name.clone())
        .collect::<Vec<String>>()
}

#[poise::command(prefix_command, slash_command, rename="concept")]
pub async fn api_concept (
    ctx: Context<'_>,
    #[description = "Search term"]
    #[autocomplete = "autocomplete_concept"]
    #[rename = "concept"]
    concept_search: String,
) -> Result<(), Error> {

    let cache = ctx.data().apicache.clone();
    let api = cache.read().unwrap().clone();

    let search_result = api.concepts.iter().find(|concept| concept_search.eq_ignore_ascii_case(&concept.common.name)).unwrap();
    let builder = CreateReply::default()
        .embed(search_result.to_embed().await);
    ctx.send(builder).await?;
    Ok(())
}

async fn autocomplete_concept<'a>(
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let cache = ctx.data().apicache.clone();
    let api = cache.read().unwrap().clone();
    api.concepts.iter()
        .filter(|c| c.common.name.to_lowercase().starts_with(&partial.to_lowercase()))
        .map(|c| c.common.name.clone())
        .collect::<Vec<String>>()
}

#[poise::command(prefix_command, slash_command, rename="builtin_type")]
pub async fn api_builtintype (
    ctx: Context<'_>,
    #[description = "Search term"]
    #[autocomplete = "autocomplete_builtintype"]
    #[rename = "builtin_type"]
    builtintype_search: String,
) -> Result<(), Error> {

    let cache = ctx.data().apicache.clone();
    let api = cache.read().unwrap().clone();

    let search_result = api.builtin_types.iter().find(|builtin_type| builtintype_search.eq_ignore_ascii_case(&builtin_type.common.name)).unwrap();
    let builder = CreateReply::default()
        .embed(search_result.to_embed().await);
    ctx.send(builder).await?;
    Ok(())
}

async fn autocomplete_builtintype<'a> (
    ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String>{
    let cache = ctx.data().apicache.clone();
    let api = cache.read().unwrap().clone();
    api.builtin_types.iter()
        .filter(|c| c.common.name.to_lowercase().starts_with(&partial.to_lowercase()))
        .map(|c| c.common.name.clone())
        .collect::<Vec<String>>()
}