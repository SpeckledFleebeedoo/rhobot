pub mod data;
pub mod runtime;

use data::api_data;
use runtime::api_runtime;
use poise::serenity_prelude as serenity;
use poise::reply::CreateReply;

use crate::{Context, Error};

/// Link a page in the mod making API. Slash commands only.
#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, track_edits, subcommands("api_runtime", "api_data", "api_page"))]
pub async fn api(
    _ctx: Context<'_>
) -> Result<(), Error> {
    Ok(())
}

#[derive(Debug, poise::ChoiceParameter)]
enum ApiPage{
    Home,
    Lifecycle,
    #[name = "Libraries and Functions"]
    Libraries,
    Classes,
    Events,
    Concepts,
    Defines,
    Prototypes,
    Types,
    #[name = "Prototype Inheritance Tree"]
    PrototypeTree
}

#[allow(clippy::unused_async)]
#[poise::command(prefix_command, slash_command, track_edits, rename="page")]
pub async fn api_page (
    ctx: Context<'_>,
    #[description = "API page to link"]
    page: ApiPage,
) -> Result<(), Error> {

    let (name, url) = match page {
    ApiPage::Home => ("Home", "https://lua-api.factorio.com/latest/"),
    ApiPage::Lifecycle => ("Lifecycle", "https://lua-api.factorio.com/latest/auxiliary/data-lifecycle.html"),
    ApiPage::Libraries => ("Libraries and Functions", "https://lua-api.factorio.com/latest/auxiliary/libraries.html"),
    ApiPage::Classes => ("Classes", "https://lua-api.factorio.com/latest/classes.html"),
    ApiPage::Events => ("Events", "https://lua-api.factorio.com/latest/events.html"),
    ApiPage::Concepts => ("Concepts", "https://lua-api.factorio.com/latest/concepts.html"),
    ApiPage::Defines => ("Defines", "https://lua-api.factorio.com/latest/defines.html"),
    ApiPage::Prototypes => ("Prototypes", "https://lua-api.factorio.com/latest/prototypes.html"),
    ApiPage::Types => ("Types", "https://lua-api.factorio.com/latest/types.html"),
    ApiPage::PrototypeTree => ("Prototype Inheritance Tree", "https://lua-api.factorio.com/latest/tree.html"),
    };
    
    let embed = serenity::CreateEmbed::new()
        .title(name)
        .description(url)
        .color(serenity::Colour::GOLD);
    let builder = CreateReply::default()
        .embed(embed);
    ctx.send(builder).await?;
    Ok(())
}