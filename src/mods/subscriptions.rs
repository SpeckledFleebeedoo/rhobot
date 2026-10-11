use std::{borrow::Cow, time::Duration};

use crate::{
    Context, Error, database,
    management::checks::{is_mod, is_mod_interaction},
    mods::commands::autocomplete_modname,
};
use poise::CreateReply;
use poise::serenity_prelude as serenity;
use poise::serenity_prelude::{CollectComponentInteractions, futures::StreamExt};
use tracing::error;

use super::error::ModError;

/// View/manage mod and author subscriptions
#[allow(clippy::unused_async, clippy::cast_possible_wrap)]
#[poise::command(slash_command, guild_only, category = "Subscriptions")]
pub async fn subscriptions(ctx: Context<'_>) -> Result<(), Error> {
    let user_is_mod = is_mod(poise::Context::Application(ctx)).await?;
    let reply = create_subscription_manager_message(ctx, user_is_mod).await?;
    let message_handle = ctx.send(reply).await?;
    if !user_is_mod {
        return Ok(());
    }
    let message = message_handle.message().await.unwrap();
    let db = &ctx.data().database;

    let mut interaction_stream = message
        .id
        .collect_component_interactions(ctx.serenity_context())
        .timeout(Duration::from_hours(1))
        .stream();
    while let Some(interaction) = interaction_stream.next().await {
        if !is_mod_interaction(&interaction, db).await {
            let builder = serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .content("Invalid permissions")
                    .ephemeral(true),
            );
            interaction.create_response(ctx.http(), builder).await?;
            continue;
        }
        match interaction.data.custom_id.as_str() {
            "remove_mod_subscription" => {
                let modname = match &interaction.data.kind {
                    serenity::ComponentInteractionDataKind::StringSelect { values } => &values[0],
                    _ => panic!("Unexpected interaction type"),
                };
                unsubscribe_from(ctx, modname.clone(), interaction, SubscribeType::Mod).await?;
            }
            "remove_author_subscription" => {
                let author = match &interaction.data.kind {
                    serenity::ComponentInteractionDataKind::StringSelect { values } => &values[0],
                    _ => panic!("Unexpected interaction type"),
                };
                unsubscribe_from(ctx, author.clone(), interaction, SubscribeType::Author).await?;
            }
            "allow_mod" => {
                let modname = match &interaction.data.kind {
                    serenity::ComponentInteractionDataKind::StringSelect { values } => &values[0],
                    _ => panic!("Unexpected interaction type"),
                };
                unsubscribe_from(ctx, modname.clone(), interaction, SubscribeType::Exclude).await?;
            }
            _ => panic!("Unexpected interaction id"),
        }
        let new_message = create_subscription_manager_message(ctx, user_is_mod).await?;
        message_handle
            .edit(poise::Context::Application(ctx), new_message)
            .await?;
    }

    let new_message = create_subscription_manager_message(ctx, false).await?;
    message_handle
        .edit(poise::Context::Application(ctx), new_message)
        .await?;

    Ok(())
}

async fn create_subscription_manager_message(
    ctx: Context<'_>,
    with_components: bool,
) -> Result<CreateReply<'_>, Error> {
    let server = ctx.guild_id().ok_or_else(|| ModError::ServerNotFound)?;
    let server_id = server.get().cast_signed();
    let db = &ctx.data().database;
    let subscribed_mods_vec = database::get_subscribed_mods(db, server_id).await?;
    let subscribed_mods = if subscribed_mods_vec.is_empty() {
        String::from("_None_")
    } else {
        subscribed_mods_vec.join("\n")
    };
    let subscribed_authors_vec = database::get_subscribed_authors(db, server_id).await?;
    let subscribed_authors = if subscribed_authors_vec.is_empty() {
        String::from("_None_")
    } else {
        subscribed_authors_vec.join("\n")
    };
    let excluded_mods_vec = database::get_excluded_mods(db, server_id).await?;
    let excluded_mods = if excluded_mods_vec.is_empty() {
        String::from("_None_")
    } else {
        excluded_mods_vec.join("\n")
    };
    let command_id = ctx
        .serenity_context()
        .http
        .get_global_commands()
        .await?
        .into_iter()
        .find(|command| command.name == "subscribe")
        .unwrap()
        .id;
    let formatted_text = if subscribed_mods_vec.is_empty() && subscribed_authors_vec.is_empty() {
        String::from("No subscriptions, showing all mod updates")
    } else {
        format!(
            "**Subscribed mods**\n\
            {subscribed_mods}\n\n\
            **Subscribed authors**\n\
            {subscribed_authors}\n\n\
            **Excluded mods**\n\
            {excluded_mods}"
        )
    };

    let subscription_list = serenity::CreateContainerComponent::TextDisplay(
        serenity::CreateTextDisplay::new(formatted_text),
    );
    let spacer = serenity::CreateContainerComponent::Separator(serenity::CreateSeparator::new());
    let command_component = serenity::CreateContainerComponent::TextDisplay(
        serenity::CreateTextDisplay::new(format!(
            "</subscribe mod:{command_id}> </subscribe author:{command_id}> </subscribe exclude:{command_id}>"
        )),
    );
    let subscription_list_container = serenity::CreateComponent::Container(
        serenity::CreateContainer::new(vec![subscription_list, spacer, command_component])
            .accent_colour(serenity::Colour::BLURPLE),
    );
    let mut message_components = vec![subscription_list_container];

    if with_components {
        add_dropdown(
            &subscribed_mods_vec,
            &mut message_components,
            "remove_mod_subscription",
            "Select mod to unsubscribe from",
        );

        add_dropdown(
            &subscribed_authors_vec,
            &mut message_components,
            "remove_author_subscription",
            "Select author to unsubscribe from",
        );

        add_dropdown(
            &excluded_mods_vec,
            &mut message_components,
            "allow_mod",
            "Select excluded mod to allow",
        );
    }

    let flags = serenity::MessageFlags::IS_COMPONENTS_V2;
    let reply = CreateReply::new()
        .flags(flags)
        .components(message_components);
    Ok(reply)
}

fn add_dropdown<'a>(
    items: &[String],
    message_components: &mut Vec<serenity::CreateComponent<'a>>,
    custom_id: &'a str,
    label: &'a str,
) {
    if !items.is_empty() {
        let select_menu_options = items
            .iter()
            .map(|m| serenity::CreateSelectMenuOption::new(m.clone(), m.clone()))
            .take(25)
            .collect::<Vec<_>>();
        let author_select_menu = serenity::CreateSelectMenu::new(
            custom_id,
            serenity::CreateSelectMenuKind::String {
                options: Cow::Owned(select_menu_options),
            },
        )
        .placeholder(label);
        let author_dropdown = serenity::CreateComponent::ActionRow(
            serenity::CreateActionRow::SelectMenu(author_select_menu),
        );
        message_components.push(author_dropdown);
    }
}

enum SubscribeType {
    Mod,
    Author,
    Exclude,
}

async fn unsubscribe_from(
    ctx: Context<'_>,
    name: String,
    interaction: serenity::ComponentInteraction,
    kind: SubscribeType,
) -> Result<(), Error> {
    interaction
        .create_response(ctx.http(), serenity::CreateInteractionResponse::Acknowledge)
        .await?;
    let server = ctx.guild_id().ok_or_else(|| ModError::ServerNotFound)?;
    let server_id = server.get().cast_signed();
    let db = &ctx.data().database;
    match kind {
        SubscribeType::Mod => database::remove_mod_subscription(db, server_id, &name).await?,
        SubscribeType::Author => database::remove_author_subscription(db, server_id, &name).await?,
        SubscribeType::Exclude => database::remove_mod_exclusion(db, server_id, &name).await?,
    }
    Ok(())
}

/// Subscribe to a mod or author. Only subscriptions are shown in the update feed.
#[allow(clippy::unused_async)]
#[poise::command(
    slash_command,
    guild_only,
    check = "is_mod",
    subcommands("subscribe_author", "subscribe_mod", "exclude_mod"),
    subcommand_required,
    category = "Subscriptions"
)]
pub async fn subscribe(_: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Subscribe to a mod
#[allow(clippy::unused_async, clippy::cast_possible_wrap)]
#[poise::command(slash_command, guild_only, check = "is_mod", rename = "mod")]
pub async fn subscribe_mod(
    ctx: Context<'_>,
    #[description = "Name of the mod to subscribe to"]
    #[autocomplete = "autocomplete_modname"]
    modname: String,
) -> Result<(), Error> {
    let server = ctx.guild_id().ok_or_else(|| ModError::ServerNotFound)?;
    let server_id = server.get() as i64;
    let db = &ctx.data().database;
    let modname = modname.trim_start_matches('*').to_string();

    database::add_mod_subscription(db, server_id, &modname).await?;
    let response = format!("Mod {modname} added to subscriptions");
    ctx.reply(response).await?;

    Ok(())
}

/// Subscribe to a mod author
#[allow(clippy::unused_async, clippy::cast_possible_wrap)]
#[poise::command(slash_command, guild_only, check = "is_mod", rename = "author")]
pub async fn subscribe_author(
    ctx: Context<'_>,
    #[description = "Name of the mod author to subscribe to"]
    #[autocomplete = "autocomplete_author"]
    author: String,
) -> Result<(), Error> {
    let server = ctx.guild_id().ok_or_else(|| ModError::ServerNotFound)?;
    let server_id = server.get() as i64;
    let db = &ctx.data().database;

    database::add_author_subscription(db, server_id, &author).await?;
    let response = format!("Author {author} added to subscriptions");
    ctx.reply(response).await?;

    Ok(())
}

#[allow(clippy::unused_async)]
async fn autocomplete_author<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> serenity::CreateAutocompleteResponse<'a> {
    let cache = &ctx.data().mod_author_cache;
    let author_cache = match cache.read() {
        Ok(c) => c,
        Err(e) => {
            error! {"Error acquiring cache: {e}"}
            return serenity::CreateAutocompleteResponse::new();
        }
    }
    .clone();

    let choices = author_cache
        .into_iter()
        .filter(|entry| {
            entry
                .to_lowercase()
                .starts_with(partial.to_lowercase().trim())
        })
        .take(25)
        .map(serenity::AutocompleteChoice::from)
        .collect::<Vec<serenity::AutocompleteChoice>>();

    serenity::CreateAutocompleteResponse::new().set_choices(choices)
}

/// Exclude a mod from a subscribed author
#[allow(clippy::unused_async, clippy::cast_possible_wrap)]
#[poise::command(slash_command, guild_only, check = "is_mod", rename = "exclude")]
pub async fn exclude_mod(
    ctx: Context<'_>,
    #[description = "Name of the mod to exclude"]
    #[autocomplete = "autocomplete_modname"]
    modname: String,
) -> Result<(), Error> {
    let server = ctx.guild_id().ok_or_else(|| ModError::ServerNotFound)?;
    let server_id = server.get() as i64;
    let db = &ctx.data().database;
    let modname = modname.trim_start_matches('*').to_string();

    database::add_mod_exclusion(db, server_id, &modname).await?;
    let response = format!("Mod {modname} excluded from subscriptions");
    ctx.reply(response).await?;

    Ok(())
}
