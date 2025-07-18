use std::fmt;

#[derive(Debug)]
pub enum ApiError {
    Unauthorized(String),
    BadRequest(String),
    NotFound(String),
    ServerError(String),
    Unexpected(String),
    DecodeError(String),
    InvalidInput(String),
    RateLimited,
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Unauthorized(msg) => write!(f, "Unauthorized: {msg}"),
            ApiError::BadRequest(msg) => write!(f, "Bad Request: {msg}"),
            ApiError::NotFound(msg) => write!(f, "Not Found: {msg}"),
            ApiError::ServerError(msg) => write!(f, "Server Error: {msg}"),
            ApiError::Unexpected(msg) => write!(f, "Unexpected Error: {msg}"),
            ApiError::DecodeError(msg) => write!(f, "Decoding Error: {msg}"),
            ApiError::InvalidInput(msg) => write!(f, "Invalid Input: {msg}"),
            ApiError::RateLimited => {
                write!(
                    f,
                    "Consider spacing out your requests to avoid hitting the rate limit"
                )
            }
        }
    }
}

impl std::error::Error for ApiError {}
