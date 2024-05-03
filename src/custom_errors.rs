use std::{error, fmt};

#[derive(Debug, Clone)]
pub struct CustomError{
    pub msg: String,
}

impl CustomError {
    pub fn new(message: &str) -> CustomError {
        CustomError {msg: message.to_owned()}
    }
}

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl error::Error for CustomError {}