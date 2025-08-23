pub mod data;
pub mod error;
pub mod lua;
mod lua_constants;
pub mod runtime;

use data::{api_prototype, api_type};
use runtime::{api_class, api_concept, api_define, api_event};

use core::fmt;
use log::warn;
use poise::reply::CreateReply;
use poise::serenity_prelude as serenity;
use regex::Regex;
use std::sync::{Arc, RwLock};

use crate::{Context, Data, Error, SEPARATOR};
use error::ApiError;

/// Link a page in the mod making API.
#[allow(clippy::unused_async)]
#[poise::command(
    prefix_command,
    slash_command,
    track_edits,
    subcommands(
        "api_class",
        "api_event",
        "api_define",
        "api_concept",
        "api_prototype",
        "api_type",
        "api_page"
    ),
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn api(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[derive(Debug, poise::ChoiceParameter)]
enum ApiPage {
    Home,
    Lifecycle,
    Storage,
    #[name = "Mod structure"]
    Structure,
    #[name = "Changelog Format"]
    Changelog,
    Migrations,
    #[name = "Libraries and Functions"]
    Libraries,
    Classes,
    Events,
    Concepts,
    Defines,
    Prototypes,
    Types,
    #[name = "Prototype Inheritance Tree"]
    PrototypeTree,
    #[name = "Noise Expressions"]
    NoiseExpressions,
    #[name = "Instrument Mode"]
    InstrumentMode,
    #[name = "Item Weight"]
    ItemWeight,
    #[name = "Modding Tutorial"]
    ModdingTutorial,
    #[name = "Scripting Tutorial"]
    ScriptingTutorial,
    Localisation,
    #[name = "Scenario System"]
    ScenarioSystem,
    #[name = "Command Line Parameters"]
    CommandLine,
    #[name = "Console Commands"]
    Console,
    #[name = "data.raw"]
    DataRaw,
}

/// Link a page in the auxillary API docs
#[allow(clippy::unused_async)]
#[poise::command(
    prefix_command,
    slash_command,
    track_edits,
    rename = "page",
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn api_page(
    ctx: Context<'_>,
    #[description = "API page to link"] page: ApiPage,
) -> Result<(), Error> {
    let (name, url) = match page {
        ApiPage::Home => ("Home", "https://lua-api.factorio.com/latest/"),
        ApiPage::Lifecycle => (
            "Lifecycle",
            "https://lua-api.factorio.com/latest/auxiliary/data-lifecycle.html",
        ),
        ApiPage::Storage => (
            "Storage",
            "https://lua-api.factorio.com/latest/auxiliary/storage.html",
        ),
        ApiPage::Structure => (
            "Mod Structure",
            "https://lua-api.factorio.com/latest/auxiliary/mod-structure.html",
        ),
        ApiPage::Changelog => (
            "Changelog Format",
            "https://lua-api.factorio.com/latest/auxiliary/changelog-format.html",
        ),
        ApiPage::Migrations => (
            "Migrations",
            "https://lua-api.factorio.com/latest/auxiliary/migrations.html",
        ),
        ApiPage::Libraries => (
            "Libraries and Functions",
            "https://lua-api.factorio.com/latest/auxiliary/libraries.html",
        ),
        ApiPage::Classes => (
            "Classes",
            "https://lua-api.factorio.com/latest/classes.html",
        ),
        ApiPage::Events => ("Events", "https://lua-api.factorio.com/latest/events.html"),
        ApiPage::Concepts => (
            "Concepts",
            "https://lua-api.factorio.com/latest/concepts.html",
        ),
        ApiPage::Defines => (
            "Defines",
            "https://lua-api.factorio.com/latest/defines.html",
        ),
        ApiPage::Prototypes => (
            "Prototypes",
            "https://lua-api.factorio.com/latest/prototypes.html",
        ),
        ApiPage::Types => ("Types", "https://lua-api.factorio.com/latest/types.html"),
        ApiPage::PrototypeTree => (
            "Prototype Inheritance Tree",
            "https://lua-api.factorio.com/latest/tree.html",
        ),
        ApiPage::NoiseExpressions => (
            "Noise Expressions",
            "https://lua-api.factorio.com/latest/auxiliary/noise-expressions.html",
        ),
        ApiPage::InstrumentMode => (
            "Instrument Mode",
            "https://lua-api.factorio.com/latest/auxiliary/instrument.html",
        ),
        ApiPage::ItemWeight => (
            "Item Weight",
            "https://lua-api.factorio.com/latest/auxiliary/item-weight.html",
        ),
        ApiPage::ModdingTutorial => (
            "Modding Tutorial",
            "https://wiki.factorio.com/Tutorial:Modding_tutorial/Gangsir",
        ),
        ApiPage::ScriptingTutorial => (
            "Scripting Tutorial",
            "https://wiki.factorio.com/Tutorial:Scripting",
        ),
        ApiPage::Localisation => (
            "Localisation",
            "https://wiki.factorio.com/Tutorial:Localisation",
        ),
        ApiPage::ScenarioSystem => (
            "Scenario System",
            "https://wiki.factorio.com/Scenario_system",
        ),
        ApiPage::CommandLine => (
            "Command Line Parameters",
            "https://wiki.factorio.com/Command_line_parameters",
        ),
        ApiPage::Console => ("Console Commands", "https://wiki.factorio.com/Console"),
        ApiPage::DataRaw => ("data.raw", "https://wiki.factorio.com/Data.raw"),
    };

    let embed = serenity::CreateEmbed::new()
        .title(name)
        .description(url)
        .color(serenity::Colour::GOLD);
    let builder = CreateReply::default().embed(embed);
    ctx.send(builder).await?;
    Ok(())
}

#[derive(Debug)]
struct ReMatch {
    full: String,
    linktext: String,
    category: String,
    page: String,
    property: Option<String>,
}

#[derive(Debug, Default, PartialEq)]
enum ApiSection {
    Type,
    Prototype,
    Class,
    #[default]
    Other,
}

impl fmt::Display for ApiSection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Type => write!(f, "types"),
            Self::Prototype => write!(f, "prototypes"),
            Self::Class => write!(f, "classes"),
            Self::Other => write!(f, ""),
        }
    }
}

pub fn resolve_internal_links(data: &Data, s: &str) -> String {
    let link_regex = Regex::new(
        r"\[(?<linktext>.+?)\]\((?<cat>runtime|prototype):(?<page>.+?)(?<property>::.+?)?\)",
    )
    .unwrap();
    let captures = link_regex
        .captures_iter(s)
        .map(|caps| ReMatch {
            full: caps
                .get(0)
                .map(|f| f.as_str().to_owned())
                .unwrap_or_default(),
            linktext: caps
                .name("linktext")
                .map(|f| f.as_str().to_owned())
                .unwrap_or_default(),
            category: caps
                .name("cat")
                .map(|f| f.as_str().to_owned())
                .unwrap_or_default(),
            page: caps
                .name("page")
                .map(|f| f.as_str().to_owned())
                .unwrap_or_default(),
            property: caps.name("property").map(|f| f.as_str().to_owned()),
        })
        .collect::<Vec<ReMatch>>();
    let mut output: String = s.to_string();
    for capture in &captures {
        let linktext = &capture.linktext;
        let section = match capture.category.as_str() {
            "runtime" => ApiSection::Class,
            "prototype" => get_prototype_category(&data.data_api_cache, &capture.page).unwrap(),
            _ => ApiSection::default(),
        };
        if section == ApiSection::default() {
            warn!("Failed to parse internal API link: {}", capture.full);
            output = output.replace(&capture.full, linktext);
        } else {
            let name = &capture.page;
            let property_opt = &capture.property.clone().unwrap_or_default();
            let property = property_opt.trim_start_matches(':');

            output = output.replace(&capture.full, &format!("[{linktext}](https://lua-api.factorio.com/latest/{section}/{name}.html#{property})"));
        }
    }
    output
}

fn get_prototype_category(
    prototype_api_cache: &Arc<RwLock<data::ApiResponse>>,
    name: &str,
) -> Result<ApiSection, Error> {
    let api = match prototype_api_cache.read() {
        Ok(c) => c,
        Err(e) => {
            return Err(ApiError::CacheError(e.to_string()))?;
        }
    }
    .clone();

    let prototype_name = api
        .prototypes
        .iter()
        .map(|p| p.common.name.clone())
        .find(|n| n == name);
    if prototype_name.is_some() {
        return Ok(ApiSection::Prototype);
    };
    let type_name = api
        .types
        .iter()
        .map(|t| t.common.name.clone())
        .find(|n| n == name);
    if type_name.is_some() {
        return Ok(ApiSection::Type);
    };
    Ok(ApiSection::default())
}

/// Splits and sanitizes inputs that use ``item::property`` shorthand or include comments
fn split_inputs(main_search: &mut String, property_search: &mut Option<String>) {
    if main_search.contains("::") {
        let search_clone = main_search.clone();
        let parts = search_clone.split_once("::").unwrap();
        *main_search = parts.0.to_string();
        *property_search = Some(parts.1.to_string());
    }

    if let Some(property) = property_search {
        if property.contains(SEPARATOR) {
            let parts = property.split_once(SEPARATOR).unwrap(); // Safe due to if condition before
            let property = parts.0.trim().to_owned();
            if property.is_empty() {
                *property_search = None;
            } else {
                *property_search = Some(parts.0.trim().to_owned());
            }
        }
    }
}
