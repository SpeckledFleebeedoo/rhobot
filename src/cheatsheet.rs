use crate::{Context, Error};
use poise::{ChoiceParameter, CreateReply};
use poise::serenity_prelude::{Colour, CreateEmbed};

#[derive(Debug, poise::ChoiceParameter)]
enum CheatSheetPage {
    #[name = "Common Ratios"]
    CommonRatios,
    Belts,
    Balancers,
    #[name = "Material Processing"]
    MaterialProcessing,
    Mining,
    Science,
    #[name = "Steam Power"]
    SteamPower,
    #[name = "Solar Power"]
    SolarPower,
    #[name = "Nuclear Power"]
    NuclearPower,
    #[name = "Oil Refining"]
    OilRefining,
    Trains,
    #[name = "Fluid Wagon Transfer"]
    FluidWagonTransfer,
    #[name = "Cargo Wagon Transfer"]
    CargoWagonTransfer,
    #[name = "Inserter Throughput"]
    InserterThroughput,
    #[name = "Inserter Capacity Bonus"]
    InserterCapacityBonus,
    #[name = "Modules and Beacons"]
    ModulesAndBeacons,
    #[name = "Productivity Module Payoffs"]
    ProductivityModulePayoffs,
    #[name = "Vehice Fuel Bonus"]
    VehicleFuelBonus,
    #[name = "Train Colors"]
    TrainColors,
    #[name = "Space Age"]
    SpaceAge,
    Tips,
    Links,
    #[name = "Popular Mod List"]
    PopularModList
}

/// Link a Factorio Cheatsheet page
#[poise::command(
    slash_command,
    prefix_command,
    track_edits,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn cheatsheet(ctx: Context<'_>, page: CheatSheetPage) -> Result<(), Error> {
    let title = page.name();
    let url = title.to_lowercase().replace(' ', "-");

    let embed = CreateEmbed::new()
        .title(format!("Factorio Cheat Sheet: {title}"))
        .url(format!("https://factoriocheatsheet.com/#{url}"))
        .color(Colour::ORANGE);
    let builder = CreateReply::default().embed(embed);
    ctx.send(builder).await?;
    Ok(())
}