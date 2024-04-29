# Factorio Mod Notifier

## Description

Factorio Mod Notifier is Discord bot written in Rust which can send mod update notifications to Discord servers, written primarily for the [Factorio discord server](https://discord.gg/factorio).

## Features

- Automatically checks for mod updates
- Sends notifications when updates are available
- Mod search command for easily sharing mods in Discord
- Per-server subscription filters to specific mods or authors
- Customizable notification settings

## Installation

1. Clone the repository: `git clone https://github.com/SpeckledFleebeedoo/Factorio-mod-notifier-rs.git`
2. Create a .env file from the included [template](.env.template) and set a discord bot token. 
3. [Install Rust](https://www.rust-lang.org/tools/install)
4. Create a database: `sqlx database create`
5. Apply all migrations to the database: `sqlx migrate run`
3. Build the application: `cargo build`
4. Run the application: `cargo run`
