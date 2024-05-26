use crate::{Context, Error};
use poise::CreateReply;
use rand::Rng;
use tokio::time::{sleep, Duration};

/// Gives "information" about the expansion
#[allow(clippy::unused_async)]
#[poise::command(slash_command, prefix_command)]
pub async fn expansion(ctx: Context<'_>) -> Result<(), Error> {
    let random = rand::thread_rng().gen_range(0..20);
    let entities = ["assembler", "splitter", "worm", 
                                "tank", "chest", "rail signal", 
                                "hazard concrete", "solar panel", 
                                "iron ore", "steam", "belt immunity equipment"];
    match random {
        1 => {ctx.reply("There's an expansion?! Tell me more!").await?;},
        2 => {ctx.reply(format!("The expansion will be out for release in just {} minutes!", rand::thread_rng().gen_range(1..=60))).await?;},
        3 => {ctx.reply("The expansion will be released before Half-Life 3.").await?;},
        4 => {ctx.reply("The expansion will be released when it's done.").await?;},
        5 => {ctx.reply("The expansion gets delayed by a week every time you ask.").await?;},
        6..=9 => {
            ctx.reply(format!("The expansion will be released as soon as the {} rework is done", 
                entities[rand::thread_rng().gen_range(0..entities.len())]
            )).await?;
        },
        _ => {
            let msg = ctx.reply("Calculating time until expansion release... :game_die:").await?;
            sleep(Duration::from_secs(2)).await;
            let builder = CreateReply::default().content(format!("The expansion will be released in {} days.", 
                rand::thread_rng().gen_range(10..=200)
            ));
            msg.edit(ctx, builder).await?;
        },
    };
    Ok(())
}