pub mod checks;
pub mod commands;

use crate::{
    Context,
    Error,
    custom_errors::CustomError,
};

#[allow(clippy::cast_possible_wrap)]
pub fn get_server_id(ctx: Context<'_>) -> Result<i64, Error> {
    let Some(server) = ctx.guild_id() else {
        return Err(Box::new(CustomError::new("Could not get server ID")))
    };
    Ok(server.get() as i64)
}