use log::{error, warn, info};
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

#[allow(clippy::single_match_else)]
impl RhobotError {
    pub fn log(&self) {
        match &self {
            Self::FFF(ffferror) => {
                match ffferror {
                    fff_commands::FFFError::PageNotFound(_) => info!("{ffferror}"),
                    fff_commands::FFFError::BadStatusCode(_) => warn!("{ffferror}"),
                    _ => error!("{ffferror}"),
                }
            },
            Self::FAQ(faq_error) => 
                match faq_error {
                    faq_commands::FaqError::NotInDatabase(_) |
                    faq_commands::FaqError::NotFound(_) |
                    faq_commands::FaqError::TitleTooLong |
                    faq_commands::FaqError::BodyTooLong |
                    faq_commands::FaqError::ServerNotFound |
                    faq_commands::FaqError::EmbedNotFound |
                    faq_commands::FaqError::EmbedContainsNoImage |
                    faq_commands::FaqError::AlreadyExists(_) |
                    faq_commands::FaqError::NotOwner => info!("{faq_error}"),
                    _ => error!("{faq_error}"),
                },
            Self::Management(management_error) => error!("{management_error}"),
            Self::API(api_error) => {
                match api_error {
                    modding_api::error::ApiError::PrototypeNotFound(_) |
                    modding_api::error::ApiError::TypeNotFound(_) |
                    modding_api::error::ApiError::ClassNotFound(_) |
                    modding_api::error::ApiError::EventNotFound(_) |
                    modding_api::error::ApiError::DefineNotFound(_) |
                    modding_api::error::ApiError::ConceptNotFound(_) |
                    modding_api::error::ApiError::LuaChapterNotFound(_) |
                    modding_api::error::ApiError::LuaFunctionNotFound(_) => info!("{api_error}"),
                    modding_api::error::ApiError::BadStatusCode(_) => warn!("{api_error}"),
                    _ => error!("{api_error}")
                }
            },
            Self::Mod(mod_error) => {
                match mod_error {
                    mods::error::ModError::ModNotFound(_) => info!("{mod_error}"),
                    mods::error::ModError::BadStatusCode(_) => warn!("{mod_error}"),
                    _ => error!{"{mod_error}"}
                }
            },
            Self::Database(database_error) => error!("{database_error}"),
            Self::Wiki(wiki_error) => {
                match wiki_error {
                    wiki_commands::WikiError::NoSearchResults(_) => info!("{wiki_error}"),
                    _ => error!("{wiki_error}")
                }
            },
            Self::Serenity(error) => error!("{error}"),
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
