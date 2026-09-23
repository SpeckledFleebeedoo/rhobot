use std::{error, fmt};

use crate::database::DatabaseError;

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub enum ModError {
    ReqwestError(Box<reqwest::Error>),
    ServerNotFound,
    CacheError(String),
    ModNotFound(String),
    BadStatusCode(String),
    DatabaseError(DatabaseError),
    ChangelogNotFound(String),
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
            Self::ChangelogNotFound(version) => f.write_str(&format!("Could not find a changelog for version {version}"))
        }
    }
}

impl error::Error for ModError {}

impl From<reqwest::Error> for ModError {
    fn from(value: reqwest::Error) -> Self {
        Self::ReqwestError(Box::new(value))
    }
}

impl From<DatabaseError> for ModError {
    fn from(value: DatabaseError) -> Self {
        Self::DatabaseError(value)
    }
}
