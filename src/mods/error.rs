use std::{error, fmt};

use crate::database::DatabaseError;

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub enum ModError {
    ReqwestError(reqwest::Error),
    ServerNotFound,
    CacheError(String),
    ModNotFound(String),
    BadStatusCode(String),
    DatabaseError(DatabaseError),
}

impl fmt::Display for ModError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::ReqwestError(error) => f.write_str(&format!("Reqwest error: {error}.")),
            Self::ServerNotFound => f.write_str("Could not retrieve server data."),
            Self::CacheError(error) => f.write_str(&format!("Error acquiring cache: {error}")),
            Self::ModNotFound(modname) => {
                f.write_str(&format!("Did not find any mods named {modname}"))
            }
            Self::BadStatusCode(status) => f.write_str(&format!(
                "Received HTTP status code {status} while accessing mod portal."
            )),
            Self::DatabaseError(error) => f.write_str(&format!("Mod database error: {error}")),
        }
    }
}

impl error::Error for ModError {}

impl From<reqwest::Error> for ModError {
    fn from(value: reqwest::Error) -> Self {
        Self::ReqwestError(value)
    }
}

impl From<DatabaseError> for ModError {
    fn from(value: DatabaseError) -> Self {
        Self::DatabaseError(value)
    }
}
