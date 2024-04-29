mod mod_commands;
mod mods;
mod faq_commands;
mod custom_errors;
mod util;

use mods::{get_mod_count, update_database, update_mod_cache, update_sub_cache, update_author_cache, ModCacheEntry, SubCacheEntry};
use faq_commands::{update_faq_cache, FaqCacheEntry};
use tokio::time;
use log::{ error, info};
use dotenv::dotenv;

use poise::serenity_prelude as serenity;
use std::{
    env::var,
    sync::{Arc, RwLock},
    time::Duration,
};

// Types used by all command functions
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

// Custom user data passed to all command functions
pub struct Data {
    database: sqlx::SqlitePool,
    modcache: Arc<RwLock<Vec<ModCacheEntry>>>,
    faqcache: Arc<RwLock<Vec<FaqCacheEntry>>>,
    subscriptioncache: Arc<RwLock<Vec<SubCacheEntry>>>,
    authorcache: Arc<RwLock<Vec<String>>>,
}

async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    // This is our custom error handler
    // They are many errors that can occur, so we only handle the ones we want to customize
    // and forward the rest to the default handler
    match error {
        poise::FrameworkError::Setup { error, .. } => panic!("Failed to start bot: {:?}", error),
        poise::FrameworkError::Command { error, ctx, .. } => {
            println!("Error in command `{}`: {:?}", ctx.command().name, error,);
        }
        poise::FrameworkError::CommandCheckFailed { ctx, .. } => {
            let _ = ctx.say("Error: invalid permissions.").await;
        }
        error => {
            if let Err(e) = poise::builtins::on_error(error).await {
                println!("Error while handling error: {}", e)
            }
        }
    }
}

// async fn on_message(ctx: Context<'_>) {

// }

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

    // FrameworkOptions contains all of poise's configuration option in one struct
    // Every option can be omitted to use its default value
    let options = poise::FrameworkOptions {
        commands: vec![
            util::help(),
            util::get_server_info(),
            util::reset_server_settings(),
            // util::migrate_serverdb_entry(),
            mod_commands::find_mod(),
            mod_commands::show_subscriptions(),
            mod_commands::subscribe(),
            mod_commands::unsubscribe(),
            mod_commands::set_updates_channel(),
            mod_commands::set_modrole(),
            mod_commands::show_changelogs(),
            faq_commands::faq(),
            faq_commands::faq_edit(),
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
        event_handler: |_ctx, event, _framework, data| {
            Box::pin(async move {
                // println!(
                //     "Got an event in event handler: {:?}",
                //     event.snake_case_name()
                // );
                match event {
                    serenity::FullEvent::GuildDelete { incomplete, full: _ } => {
                        if incomplete.unavailable == false {
                            util::on_guild_leave(incomplete.id, data.database.clone()).await?;
                        }
                    },
                    _ => {},
                }
                Ok(())
            })
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                println!("Logged in as {}", _ready.user.name);
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    database: db_clone,
                    modcache: mods_cache_clone,
                    faqcache: faq_cache_clone,
                    subscriptioncache: subscription_cache_clone,
                    authorcache: authorname_cache_clone,
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
            Ok(_) => info!{"Initialized mod database"},
            Err(error) => error!("Error while updating mod database: {error}")
        }
    }
    
    let db_clone_2 = db.clone();
    let mut mod_update_interval = time::interval(time::Duration::from_secs(60));
    tokio::spawn(async move {
        loop {
            mod_update_interval.tick().await;
            println!("Start updating mod database");
            let result = update_database(db_clone_2.clone(), &http_clone, false).await;
            match result {
                Ok(_) => info!{"Updated mod database"},
                Err(error) => error!("Error while updating mod database: {error}")
            }
        }
    });

    let mut cache_update_interval = time::interval(time::Duration::from_secs(5*60));
    tokio::spawn(async move {
        loop {
            cache_update_interval.tick().await;
            match update_mod_cache(mods_cache.clone(), db.clone()).await {
                Ok(_) => info!("Updated mod cache"),
                Err(error) => error!("Error while updating mod cache: {error}"),
            };
            match update_faq_cache(faq_cache.clone(), db.clone()).await {
                Ok(_) => info!("Updated faq cache"),
                Err(error) => error!("Error while updating faq cache: {error}"),
            };
            match update_sub_cache(subscription_cache.clone(), db.clone()).await {
                Ok(_) => info!("Updated subscription cache"),
                Err(error) => error!("Error while updating subscription cache: {error}"),
            }
            match update_author_cache(authorname_cache.clone(), db.clone()).await {
                Ok(_) => info!("Updated subscription cache"),
                Err(error) => error!("Error while updating author name cache: {error}"),
            }
            println!("Caches updated")
        };
    });

    client.unwrap().start().await.unwrap()
}
