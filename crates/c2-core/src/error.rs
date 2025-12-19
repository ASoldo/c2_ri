use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    InvalidInput,
    NotFound,
    Unauthorized,
    Forbidden,
    Conflict,
    Timeout,
    Unavailable,
    Upstream,
    Internal,
}

#[derive(Debug, Clone)]
pub struct C2Error {
    pub code: ErrorCode,
    pub message: String,
}

impl C2Error {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for C2Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for C2Error {}

pub type C2Result<T> = Result<T, C2Error>;
