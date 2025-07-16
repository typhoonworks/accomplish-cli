use crate::api::errors::ApiError;
use config::ConfigError;
use serde_json::Error as SerdeJsonError;
use thiserror::Error;

/// Central error type for the CLI.
#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("API error: {0}")]
    Api(#[from] ApiError),

    #[error("JSON error: {0}")]
    Json(#[from] SerdeJsonError),

    #[error(transparent)]
    Auth(#[from] UnauthenticatedError),

    #[error("Failed to receive authorization code callback")]
    Callback,

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("{0}")]
    Other(String),
}

/// Error indicating the user is not authenticated.
#[derive(Error, Debug)]
#[error("User is not authenticated. Please log in.")]
pub struct UnauthenticatedError;
