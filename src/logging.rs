use poise::CreateReply;
use poise::serenity_prelude as serenity;
use tracing::{info, error};
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
                .with_filter(LevelFilter::INFO)
        )
        .with(
            // Log to STDOUT
            tracing_subscriber::fmt::layer().with_filter(LevelFilter::WARN)
        )
        .init();

    info!("Logging started");
    guard
}

/// Show log files
#[poise::command(slash_command, owners_only, hide_in_help)]
pub async fn logs(ctx: Context<'_>,
    #[autocomplete = "autocomplete_logfile"]
    file: Option<String>,
) -> Result<(), Error> {
    let filename = file.unwrap_or_else(|| format!("{}.{LOG_FILENAME}", chrono::Utc::now().format("%Y-%m-%d")));
    let logfile = tokio::fs::File::open(&filename).await?;
    let attachment = serenity::CreateAttachment::file(&logfile, filename);
    let builder = CreateReply::new().attachment(attachment).ephemeral(true);
    ctx.send(builder).await?;
    Ok(())
}

async fn autocomplete_logfile<'a>(_ctx: Context<'a>, _partial: &'a str) -> serenity::CreateAutocompleteResponse<'a> {
    let mut filenames = Vec::new();
    let mut directory = match tokio::fs::read_dir(".").await {
        Ok(dir) => dir,
        Err(e) => {
            error!("Error reading directory: {e}");
            return serenity::CreateAutocompleteResponse::new();
        },
    };
    while let Ok(Some(file)) = directory.next_entry().await {
        if file.file_name().to_string_lossy().ends_with(".log") {
            filenames.push(file.file_name());
        }
    }

    let choices = filenames.clone().iter()
        .take(25)
        .map(|f| serenity::AutocompleteChoice::from(f.to_string_lossy().into_owned()))
        .collect::<Vec<serenity::AutocompleteChoice>>();

    serenity::CreateAutocompleteResponse::new().set_choices(choices)
}