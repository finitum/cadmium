use users::User;
use crate::displayservers::x::XError;
use std::error::Error;
use std::fmt;
use std::fmt::Debug;

pub mod x;

#[derive(Debug)]
pub enum DisplayServerError {
    XError(XError)
}

impl Error for DisplayServerError {}
impl fmt::Display for DisplayServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        <dyn Debug>::fmt(self, f)
    }
}

pub trait DisplayServer {
    fn pre_suid(&mut self) -> Result<(), DisplayServerError>;
    fn post_suid(&mut self, user_info: &User, de: &str) -> Result<(), DisplayServerError>;
}

impl From<XError> for DisplayServerError {
    fn from(error: XError) -> Self {
        Self::XError(error)
    }
}

