use std::{error, fmt};
use poise::serenity_prelude as serenity;
use poise::CreateReply;

use crate::{
    Context,
    Error,
};

#[derive(Debug, Clone)]
pub struct CustomError{
    pub msg: String,
}

impl CustomError {
    pub fn new(message: &str) -> Self {
        Self {msg: message.to_owned()}
    }
}

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl error::Error for CustomError {}


pub async fn send_custom_error_message(ctx: Context<'_>, msg: &str) -> Result<(), Error> {
    let embed = serenity::CreateEmbed::new()
        .title(format!("Error while executing command {}:", ctx.command().name))
        .description(msg)
        .color(serenity::Colour::RED);
    let builder = CreateReply::default()
        .embed(embed);
    ctx.send(builder).await?;
    Ok(())
}