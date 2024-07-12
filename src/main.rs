#![allow(clippy::unwrap_used, clippy::expect_used)]

mod mods;
mod faq_commands;
mod fff_commands;
mod fun_commands;
mod modding_api;
mod wiki_commands;
mod custom_errors;
mod util;

use clokwerk::{AsyncScheduler, Job};
use tokio::time;
use log::{error, info};
use dotenv::dotenv;
use poise::serenity_prelude as serenity;
use std::{
    env::var,
    sync::{Arc, RwLock},
    time::Duration,
};

use crate::{
    faq_commands::{update_faq_cache, FaqCacheEntry}, 
    fff_commands::update_fff_channel_description,
    mods::{
        update_notifications::{
            get_mod_count, 
            update_database, 
            update_mod_cache, 
            update_sub_cache, 
            update_author_cache, 
            ModCacheEntry, 
            SubCacheEntry},
        search_api::ModPortalCredentials
    },
};

// Types used by all command functions
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

// Command separator for adding comments
const SEPARATOR: char = '|';

// Custom user data passed to all command functions
pub struct Data {
    database: sqlx::SqlitePool,
    mod_cache: Arc<RwLock<Vec<ModCacheEntry>>>,
    faq_cache: Arc<RwLock<Vec<FaqCacheEntry>>>,
    mod_subscription_cache: Arc<RwLock<Vec<SubCacheEntry>>>,
    mod_author_cache: Arc<RwLock<Vec<String>>>,
    runtime_api_cache: Arc<RwLock<modding_api::runtime::ApiResponse>>,
    data_api_cache: Arc<RwLock<modding_api::data::ApiResponse>>,
    mod_portal_credentials: Arc<ModPortalCredentials>,
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {error}"),
        poise::FrameworkError::Command { error, ctx, .. } => {
            error!("Error in command `{}`: {}", ctx.command().name, error,);
            let _ = util::send_custom_error_message(ctx, &format!("{error}")).await;
        }
        poise::FrameworkError::CommandCheckFailed { ctx, .. } => {
            let _ = util::send_custom_error_message(ctx, "invalid permissions").await;
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                error!("Error while handling error: {e}");
            }
        }
    }
}

#[allow(clippy::too_many_lines, clippy::unreadable_literal)]
#[tokio::main]
async fn main() {

    env_logger::init();
    dotenv().ok();

    // Initialize sqlx database
    let db = sqlx::SqlitePool::connect(
            &var("DATABASE_URL")
            .expect("Database URL not found in environment variables")
        )
        .await
        .expect("Couldn't connect to database");
    sqlx::migrate!("./migrations").run(&db).await.expect("Couldn't run database migrations");

    let db_clone = db.clone();

    let mods_cache = Arc::new(RwLock::new(Vec::new()));
    let mods_cache_clone = mods_cache.clone();

    let faq_cache = Arc::new(RwLock::new(Vec::new()));
    let faq_cache_clone = faq_cache.clone();

    let subscription_cache = Arc::new(RwLock::new(Vec::new()));
    let subscription_cache_clone = subscription_cache.clone();

    let authorname_cache = Arc::new(RwLock::new(Vec::new()));
    let authorname_cache_clone = authorname_cache.clone();
    
    let runtime_api: modding_api::runtime::ApiResponse = match modding_api::runtime::get_runtime_api().await {
        Ok(a) => a,
        Err(e) => {
            error!("Failed to get modding runtime api: {e}");
            return
        },
    };
    let runtime_api_cache = Arc::new(RwLock::new(runtime_api));
    let runtime_api_cache_clone = runtime_api_cache.clone();

    let datastage_api: modding_api::data::ApiResponse = match modding_api::data::get_data_api().await {
        Ok(a) => a,
        Err(e) => {
            error!("Failed to get modding data api: {e}");
            return
        },
    };
    let data_api_cache = Arc::new(RwLock::new(datastage_api));
    let data_api_cache_clone = data_api_cache.clone();

    let mod_portal_cred = {
        let username = var("MOD_PORTAL_USERNAME").expect("Could not find mod portal username in .env file");
        let token = var("MOD_PORTAL_TOKEN").expect("Could not find mod portal token in .env file");
        Arc::new(ModPortalCredentials::new(username, token))
    };

    // FrameworkOptions contains all of poise's configuration option in one struct
    // Every option can be omitted to use its default value
    let options = poise::FrameworkOptions {
        commands: vec![
            util::help(),
            util::get_server_info(),
            util::reset_server_settings(),
            mods::commands::find_mod(),
            mods::commands::show_subscriptions(),
            mods::commands::subscribe(),
            mods::commands::unsubscribe(),
            mods::commands::set_updates_channel(),
            mods::commands::set_modrole(),
            mods::commands::show_changelogs(),
            faq_commands::faq(),
            faq_commands::faq_edit(),
            fff_commands::fff(),
            modding_api::api(),
            wiki_commands::wiki(),
            fun_commands::expansion(),
            util::import_legacy_faqs(),
            util::drop_faqs(),
        ],
        prefix_options: poise::PrefixFrameworkOptions {
            prefix: Some("+".into()),
            edit_tracker: Some(Arc::new(poise::EditTracker::for_timespan(
                Duration::from_secs(3600),
            ))),
            ..Default::default()
        },
        // The global error handler for all error cases that may occur
        on_error: |error| Box::pin(on_error(error)),
        // Every command invocation must pass this check to continue execution
        command_check: Some(|ctx| {
            Box::pin(async move {
                if ctx.author().id == 896387132648730684 { // Bot ID
                    return Ok(false);
                }
                Ok(true)
            })
        }),
        // Enforce command checks even for owners (enforced by default)
        // Set to true to bypass checks, which is useful for testing
        skip_checks_for_owners: false,
        event_handler: |ctx, event, _framework, data| {
            Box::pin(async move {
                if let serenity::FullEvent::GuildDelete { incomplete, full: _} = event {
                    if !incomplete.unavailable {
                        util::on_guild_leave(incomplete.id, data.database.clone()).await?;
                    }
                }
                if let serenity::FullEvent::Message { new_message } = event {
                    util::on_message(ctx.clone(), new_message, data).await?;
                }
                Ok(())
            })
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    database: db_clone,
                    mod_cache: mods_cache_clone,
                    faq_cache: faq_cache_clone,
                    mod_subscription_cache: subscription_cache_clone,
                    mod_author_cache: authorname_cache_clone,
                    runtime_api_cache: runtime_api_cache_clone,
                    data_api_cache: data_api_cache_clone,
                    mod_portal_credentials: mod_portal_cred,
                })
            })
        })
        .options(options)
        .build();

    let token = var("DISCORD_TOKEN")
        .expect("Missing `DISCORD_TOKEN` env var, see README for more information.");
    let intents =
        serenity::GatewayIntents::non_privileged() | serenity::GatewayIntents::MESSAGE_CONTENT;

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    let http_clone = client.as_ref().unwrap().http.clone();

    let mods_count = get_mod_count(db.clone()).await;
    if mods_count == 0 {
        println!("Start initializing mod database");
        let result = update_database(db.clone(), &http_clone, true).await;
        match result {
            Ok(()) => info!{"Initialized mod database"},
            Err(error) => error!("Error while updating mod database: {error}")
        }
    }
    
    let db_clone_2 = db.clone();
    let mut mod_update_interval = time::interval(time::Duration::from_secs(60));    // Update every minute
    tokio::spawn(async move {
        loop {
            mod_update_interval.tick().await;
            let result = update_database(db_clone_2.clone(), &http_clone, false).await;
            match result {
                Ok(()) => info!{"Updated mod database"},
                Err(error) => error!("Error while updating mod database: {error}")
            }
        }
    });

    let mut cache_update_interval = time::interval(time::Duration::from_secs(5*60));    // Update every 5 minutes
    tokio::spawn(async move {
        loop {
            cache_update_interval.tick().await;
            match update_mod_cache(mods_cache.clone(), db.clone()).await {
                Ok(()) => info!("Updated mod cache"),
                Err(error) => error!("Error while updating mod cache: {error}"),
            };
            match update_faq_cache(faq_cache.clone(), db.clone()).await {
                Ok(()) => info!("Updated faq cache"),
                Err(error) => error!("Error while updating faq cache: {error}"),
            };
            match update_sub_cache(subscription_cache.clone(), db.clone()).await {
                Ok(()) => info!("Updated subscription cache"),
                Err(error) => error!("Error while updating subscription cache: {error}"),
            };
            match update_author_cache(authorname_cache.clone(), db.clone()).await {
                Ok(()) => info!("Updated subscription cache"),
                Err(error) => error!("Error while updating author name cache: {error}"),
            };
            info!("Caches updated");
        };
    });

    let mut api_update_interval = time::interval(time::Duration::from_secs(60*60*24));  // Update once per day
    api_update_interval.tick().await;   // First tick happens instantly
    tokio::spawn(async move {
        loop {
            api_update_interval.tick().await;
            match modding_api::runtime::update_api_cache(runtime_api_cache.clone()).await {
                Ok(()) => info!("Updated API cache"),
                Err(error) => error!("Error while updating runtime api cache: {error}"),
            };
            match modding_api::data::update_api_cache(data_api_cache.clone()).await {
                Ok(()) => info!("Updated API cache"),
                Err(error) => error!("Error whille updating data api cache: {error}")
            }
        };
    });
    
    let http_clone = client.as_ref().unwrap().http.clone();
    // let _ = update_fff_channel_description(http_clone.clone()).await;
    let mut scheduler: AsyncScheduler = AsyncScheduler::new();
    scheduler.every(clokwerk::Interval::Friday)
        .at("12:02")
        .run(move || update_fff_channel_description(http_clone.clone()));
    
    tokio::spawn(async move {
        loop{
            scheduler.run_pending().await;
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
    });

    client.unwrap().start().await.unwrap();
}
