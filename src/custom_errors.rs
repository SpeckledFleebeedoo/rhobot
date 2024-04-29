use std::{error, fmt};

#[derive(Debug, Clone)]
pub struct StatusCodeError{
    pub msg: String,
}

impl StatusCodeError {
    pub fn new(message: &str) -> StatusCodeError {
        StatusCodeError {msg: message.to_owned()}
    }
}

impl fmt::Display for StatusCodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl error::Error for StatusCodeError {}