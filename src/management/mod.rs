pub mod checks;
pub mod commands;

use std::{fmt, error};

use crate::{
    Context,
    database::DatabaseError,
};

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub enum ManagementError {
    ServerNotFound,
    DatabaseError(DatabaseError),
    SerenityError(serenity::Error),
    OwnerVerificationFailed,
}

impl fmt::Display for ManagementError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ServerNotFound => f.write_str("Could not retrieve server data."),
            Self::DatabaseError(error) => f.write_str(&format!("Modrole database error: {error}")),
            Self::SerenityError(error) => f.write_str(&format!("Serenity error: {error}")),
            Self::OwnerVerificationFailed => f.write_str("Failed to verify if user is owner"),
        }
    }
}

impl From<DatabaseError> for ManagementError {
    fn from(value: DatabaseError) -> Self {
        Self::DatabaseError(value)
    }
}

impl From<serenity::Error> for ManagementError {
    fn from(value: serenity::Error) -> Self {
        Self::SerenityError(value)
    }
}

impl error::Error for ManagementError {}

#[allow(clippy::cast_possible_wrap)]
pub fn get_server_id(ctx: Context<'_>) -> Result<i64, ManagementError> {
    let server = ctx.guild_id().ok_or_else(|| ManagementError::ServerNotFound)?;
    Ok(server.get() as i64)
}