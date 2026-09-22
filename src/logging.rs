use poise::CreateReply;
use poise::serenity_prelude as serenity;
use tracing::info;
use tracing_appender::{
    non_blocking::WorkerGuard,
    rolling::{RollingFileAppender, Rotation},
};
use tracing_subscriber::{
    Layer, filter::LevelFilter, layer::SubscriberExt, util::SubscriberInitExt,
};

use crate::{Context, Error};

pub const LOG_FILENAME: &str = "rhobot.log";

pub fn init_logging() -> WorkerGuard {
    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_suffix(LOG_FILENAME)
        .max_log_files(5)
        .build(".")
        .expect("Failed to initialize log file");
    let (non_blocking_writer, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(
            // Log to file
            tracing_subscriber::fmt::layer()
                .with_ansi(false)
                .with_writer(non_blocking_writer)
                .with_filter(LevelFilter::INFO),
        )
        .with(
            // Log to STDOUT
            tracing_subscriber::fmt::layer().with_filter(LevelFilter::ERROR),
        )
        .init();

    info!("Logging started");
    guard
}

/// Show log files
#[poise::command(slash_command, owners_only, hide_in_help)]
pub async fn logs(ctx: Context<'_>) -> Result<(), Error> {
    let date = time::OffsetDateTime::now_utc();
    let date_formatted = date
        .format(time::macros::format_description!("[year]-[month]-[day]"))
        .expect("Failed to format date");
    let filename = format!("{date_formatted}.{LOG_FILENAME}");
    let logfile = tokio::fs::File::open(&filename).await?;
    let attachment = serenity::CreateAttachment::file(&logfile, filename);
    let builder = CreateReply::new().attachment(attachment).ephemeral(true);
    ctx.send(builder).await?;
    Ok(())
}
