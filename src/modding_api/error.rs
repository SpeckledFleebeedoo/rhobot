use crate::database;
use std::{fmt, error};

#[allow(clippy::module_name_repetitions)]
#[derive(Debug)]
pub enum ApiError{
    CacheError(String),
    DatabaseError(database::DatabaseError),
    SerenityError(serenity::Error),
    BadStatusCode(String),
    PrototypeNotFound(String),
    TypeNotFound(String),
    NoTypeProperties,
    PropertyNotFound(String),
    LuaChapterNotFound(String),
    LuaFunctionNotFound(String),
    ClassNotFound(String),
    EventNotFound(String),
    DefineNotFound(String),
    ConceptNotFound(String),


}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::CacheError(error) => f.write_str(&format!("Error acquiring cache: {error}")),
            Self::DatabaseError(error) => f.write_str(&format!("FAQ database error: {error}")),
            Self::SerenityError(error) => f.write_str(&format!("Serenity error: {error}")),
            Self::BadStatusCode(status) => f.write_str(&format!("Received HTTP status code {status} while accessing mod portal API.")),
            Self::PrototypeNotFound(name) => f.write_str(&format!("Could not find prototype `{name}` in prototype API documentation")),
            Self::TypeNotFound(name) => f.write_str(&format!("Could not find type `{name}` in prototype API documentation")),
            Self::NoTypeProperties => f.write_str("Type has no properties"),
            Self::PropertyNotFound(name) => f.write_str(&format!("Could not find property `{name}`")),
            Self::LuaChapterNotFound(name) => f.write_str(&format!(r#"Could not find chapter "{name}" in lua manual"#)),
            Self::LuaFunctionNotFound(name) => f.write_str(&format!(r#"Could not find function "{name}" in lua manual"#)),
            Self::ClassNotFound(name) => f.write_str(&format!("Could not find class `{name}` in runtime API documentation")),
            Self::EventNotFound(name) => f.write_str(&format!("Could not find event `{name}` in runtime API documentation")),
            Self::DefineNotFound(name) => f.write_str(&format!("Could not find define `{name}` in runtime API documentation")),
            Self::ConceptNotFound(name) => f.write_str(&format!("Could not find concept `{name}` in runtime API documentation")),
        }
    }
}

impl error::Error for ApiError {}

impl From<database::DatabaseError> for ApiError {
    fn from(value: database::DatabaseError) -> Self {
        Self::DatabaseError(value)
    }
}

impl From<serenity::Error> for ApiError {
    fn from(value: serenity::Error) -> Self {
        Self::SerenityError(value)
    }
}