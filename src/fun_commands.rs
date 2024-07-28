use chrono::TimeZone;
use poise::CreateReply;
use rand::Rng;

use crate::{Context, Error};

/// Shows the time left until the expansion releases. Bring a calculator...
#[allow(clippy::unused_async)]
#[poise::command(slash_command, prefix_command)]
pub async fn expansion(ctx: Context<'_>) -> Result<(), Error> {
    let units = vec![
        ("train-kilometers", 13.88889), // 1.2 tiles/tick = 72 m/s = 0.072 km/s -> 13.8889 s/km
        ("train-nautical miles", 25.7202), // 0.03888 mi/s
        ("inserter swings", 0.6),
        ("fast inserter swings", 0.21667),
        ("beaconed rocket launches", 61.417),
        ("rocket launches", 340.3333),
        ("engineer-marathons", 4690.),
        ("express belt map traversals", 355_555.555),
        ("transport belt map traversals", 1_066_666.665),
        ("ticks", 1./60.),
        ("Nauvis days", 416.66),
        ("Mars days", 88_775.),
        ("dog years", 31_536_000./7.),
        ("light-megamiles", 5.368),
        ("galactic picoyears", 7450.),
        ("kermits", 864.),
        ("uranium fuel cells", 200.),
        ("fortnights", 1_209_600.),
        ("milligenerations", 2_332_800.),
        ("kilominutes", 60000.),
        ("centiyears", 315_569.5),
        ("viewings of Star Wars Episodes I-IX", 74520.),
        ("megaseconds", 1_000_000.),
        ("kilowarhols", 900_000.),
        ("radon-222 half-lives", 330_350.),
        ("FFFs", 604_800.)
    ];

    let random = rand::thread_rng().gen_range(0..units.len());
    let (unit, conversion) = units[random];
    let mut message = time_left_message(unit, conversion);
    let mut previous_message = message.clone();
    let handle = ctx.say(&message).await?;

    // Edit message with updated timestamps for half a minute after sending
    for _ in 0..=20 {
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        message = time_left_message(unit, conversion);
        if message != previous_message {
            let builder = CreateReply::default().content(&message);
            handle.edit(ctx, builder).await?;
            previous_message.clone_from(&message);
        }
    };
    Ok(())
}

#[allow(clippy::cast_precision_loss)]
fn time_left_message (unit: &str, conversion: f64) -> String {
    let time_to_release = time_until_release() as f64;
    let duration = time_to_release / conversion;
    if duration > 10. {
        format!("The expansion will release in {duration:.1} {unit}")
    } else {
        format!("The expansion will release in {duration:.3} {unit}")
    }
}

pub fn time_until_release() -> i64 {
    let release_date = chrono::Utc.with_ymd_and_hms(2024, 10, 21, 12, 00, 00).unwrap();
    let now = chrono::Utc::now();

    (release_date - now).num_seconds()
}