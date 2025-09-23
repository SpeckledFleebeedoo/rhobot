use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, TimeZone, Utc};
use chrono_tz::Europe::Prague;

use crate::{Context, Error};

/// Shows the time left until the 2.0 anniversary.
#[allow(clippy::unused_async)]
#[poise::command(slash_command, prefix_command)]
pub async fn anniversary(ctx: Context<'_>) -> Result<(), Error> {
    let message = time_left_message();
    ctx.say(&message).await?;
    Ok(())
}

#[allow(clippy::cast_precision_loss)]
fn time_left_message () -> String {
    let now = Utc::now();
    let mut time_to_anniversary = time_until_anniversary(now.year());
    if time_to_anniversary.num_seconds() <= 0 {
        time_to_anniversary = time_until_anniversary(now.year() + 1);
    }
    let days = time_to_anniversary.num_days();
    let hours = (time_to_anniversary - TimeDelta::days(days)).num_hours();
    let minutes = (time_to_anniversary - TimeDelta::days(days) - TimeDelta::hours(hours)).num_minutes();
    let seconds = (time_to_anniversary - TimeDelta::days(days) - TimeDelta::hours(hours) - TimeDelta::minutes(minutes)).num_seconds();
    format!("Space Age anniversary is in {days} days, {hours} hours, {minutes} minutes and {seconds} seconds!")
}

pub fn time_until_anniversary(year: i32) -> chrono::TimeDelta {
    let anniversary_date = NaiveDate::from_ymd_opt(year, 10, 21).unwrap();
    let anniversary_time = NaiveTime::from_hms_opt(13, 00, 00).unwrap();
    let prague_anniversary_datetime = Prague.from_local_datetime(&NaiveDateTime::new(anniversary_date, anniversary_time)).unwrap();
    prague_anniversary_datetime.with_timezone(&Utc) - Utc::now()
}