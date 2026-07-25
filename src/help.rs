use crate::{Context, Data, Error};
use poise::CreateReply;
use poise::serenity_prelude::{AutocompleteChoice, CreateAutocompleteResponse};
use std::borrow::Cow;
use std::fmt::Write as _;

/// Show this help menu
#[allow(clippy::unused_async, clippy::option_if_let_else)]
#[poise::command(prefix_command, track_edits, slash_command)]
pub async fn help(
    ctx: Context<'_>,
    #[description = "Specific command to show help about"]
    #[autocomplete = "autocomplete_command"]
    #[rest]
    command: Option<String>,
) -> Result<(), Error> {
    let message = match command {
        Some(c) => command_details(ctx, &c)?,
        None => command_overview(ctx),
    };
    let builder = CreateReply::default().content(message);
    ctx.send(builder).await?;
    Ok(())
}

#[allow(clippy::unused_async)]
pub async fn autocomplete_command<'a>(
    ctx: Context<'a>,
    partial: &'a str,
) -> CreateAutocompleteResponse<'a> {
    let mut choices = ctx
        .framework()
        .options()
        .commands
        .iter()
        .flat_map(|cmd| {
            let mut cmdvec: Vec<_> = vec![cmd.name.to_string()];
            let mut subcommands = cmd
                .subcommands
                .iter()
                .map(|scmd| format!("{} {}", cmd.name, scmd.name))
                .collect::<Vec<_>>();
            cmdvec.append(&mut subcommands);
            cmdvec
        })
        .filter(|cmd| cmd.starts_with(partial))
        .map(AutocompleteChoice::from)
        .collect::<Vec<AutocompleteChoice>>();

    choices.truncate(25);
    CreateAutocompleteResponse::new().set_choices(choices)
}

fn command_details(ctx: Context<'_>, commandname: &str) -> Result<String, Error> {
    let prefix = ctx.prefix();

    let (parentname, command) = if commandname.contains(' ') {
        let (maincommandname, subcommandname) = commandname.split_once(' ').unwrap();
        let Some(main_c) = ctx
            .framework()
            .options()
            .commands
            .iter()
            .find(|c| c.name == maincommandname)
        else {
            return Err(Error::CommandNotFound(maincommandname.to_string()));
        };
        let Some(sub_c) = main_c.subcommands.iter().find(|c| c.name == subcommandname) else {
            return Err(Error::CommandNotFound(format!(
                "{maincommandname} {subcommandname}"
            )));
        };
        (Cow::from(format!("{} ", main_c.name)), sub_c)
    } else {
        let Some(c) = ctx
            .framework()
            .options()
            .commands
            .iter()
            .find(|c| c.name == commandname)
        else {
            return Err(Error::CommandNotFound(commandname.to_string()));
        };
        (Cow::from(""), c)
    };

    let name = format!("{parentname}{}", command.name);
    let description = command.description.clone().unwrap_or_default();

    let subcommands_list: Vec<(Cow<str>, Option<Cow<str>>)> = command
        .subcommands
        .iter()
        .map(|c| (c.name.clone(), c.description.clone()))
        .collect();

    let parameters = list_parameters(command);
    let mut message = format!("`{prefix}{name}{parameters}`\n\n{description}");

    let subcommands_text = make_two_column_list(subcommands_list, "");
    if !subcommands_text.is_empty() {
        let _ = writeln!(message, "\n```\nSubcommands:\n{subcommands_text}\n```");
    }

    let parameter_text = command
        .parameters
        .iter()
        .map(|p| {
            let required = if p.required { "" } else { "(Optional) " };
            format!(
                "{} {}{}",
                p.name,
                required,
                p.description.clone().unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    if !parameter_text.is_empty() {
        let _ = writeln!(message, "\n```\nParameters:\n{parameter_text}\n```");
    }

    Ok(message)
}

fn list_parameters(command: &poise::structs::Command<Data, Error>) -> String {
    let parameters = &command.parameters;
    let text = parameters
        .iter()
        .map(|p| {
            if p.required {
                format!("<{}>", p.name)
            } else {
                format!("<{}?>", p.name)
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    if text.is_empty() {
        return text
    }
    format!(" {text}")
}

fn command_overview(ctx: Context<'_>) -> String {
    let all_commands: &Vec<poise::Command<Data, Error>> = &ctx.framework().options().commands;
    let mut output_lines: Vec<(Cow<str>, Option<Cow<str>>)> = Vec::new();
    let mut categories = all_commands
        .iter()
        .map(|command| command.category.clone())
        .collect::<Vec<_>>();
    categories.sort();
    categories.dedup();

    for category in categories {
        let mut commands: Vec<(Cow<str>, Option<Cow<str>>)> = all_commands
            .iter()
            .filter(|c| !c.hide_in_help && category == c.category)
            .map(|c| (c.name.clone(), c.description.clone()))
            .collect();
        if !commands.is_empty() {
            output_lines.push((category.unwrap_or_else(|| Cow::from("Commands")), None));
            output_lines.append(&mut commands);
        }
    }
    let text = make_two_column_list(output_lines, ctx.prefix());

    format!("```\n{text}\n```")
}

fn make_two_column_list(entries: Vec<(Cow<str>, Option<Cow<str>>)>, prefix: &str) -> String {
    let longest_command_len = entries
        .iter()
        .filter_map(|(c, d)| if d.is_some() { Some(c.len()) } else { None })
        .max()
        .unwrap_or_default();

    let mut text = String::new();

    for (title, description) in entries {
        if let Some(d) = description {
            let padding = " ".repeat(longest_command_len - title.len() + 3);
            let _ = writeln!(text, "  {prefix}{title}{padding}{d}");
        } else {
            let _ = writeln!(text, "\n{title}: ");
        }
    }
    text
}
