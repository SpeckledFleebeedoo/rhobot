use std::fmt;

use crate::{
    fff_commands,
    faq_commands,
    management,
    modding_api,
    mods,
    database,
    wiki_commands,
};

#[allow(clippy::upper_case_acronyms, clippy::module_name_repetitions)]
#[derive(Debug)]
pub enum RhobotError {
    FFF(fff_commands::FFFError),
    FAQ(faq_commands::FaqError),
    Management(management::ManagementError),
    API(modding_api::error::ApiError),
    Mod(mods::error::ModError),
    Database(database::DatabaseError),
    Wiki(wiki_commands::WikiError),
    Serenity(serenity::Error),
}

impl fmt::Display for RhobotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FFF(error) => f.write_str(&error.to_string()),
            Self::FAQ(error) => f.write_str(&error.to_string()),
            Self::Management(error) => f.write_str(&format!("Error in Management module: {error}")),
            Self::API(error) => f.write_str(&error.to_string()),
            Self::Mod(error) => f.write_str(&error.to_string()),
            Self::Database(error) => f.write_str(&format!("Error in Database module: {error}")),
            Self::Wiki(error) => f.write_str(&error.to_string()),
            Self::Serenity(error) => f.write_str(&format!("Serenity error: {error}")),
        }
    }
}

impl std::error::Error for RhobotError {}

impl From<fff_commands::FFFError> for RhobotError {
    fn from(value: fff_commands::FFFError) -> Self {
        Self::FFF(value)
    }
}

impl From<faq_commands::FaqError> for RhobotError {
    fn from(value: faq_commands::FaqError) -> Self {
        Self::FAQ(value)
    }
}

impl From<management::ManagementError> for RhobotError {
    fn from(value: management::ManagementError) -> Self {
        Self::Management(value)
    }
}

impl From<modding_api::error::ApiError> for RhobotError{
    fn from(value: modding_api::error::ApiError) -> Self {
        Self::API(value)
    }
}

impl From<mods::error::ModError> for RhobotError {
    fn from(value: mods::error::ModError) -> Self {
        Self::Mod(value)
    }
}

impl From<database::DatabaseError> for RhobotError {
    fn from(value: database::DatabaseError) -> Self {
        Self::Database(value)
    }
}

impl From<wiki_commands::WikiError> for RhobotError {
    fn from(value: wiki_commands::WikiError) -> Self {
        Self::Wiki(value)
    }
}

impl From<serenity::Error> for RhobotError {
    fn from(value: serenity::Error) -> Self {
        Self::Serenity(value)
    }
}
