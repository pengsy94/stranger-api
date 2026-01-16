use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Configuration already initialized")]
    AlreadyInitialized,

    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),

    #[error("Invalid value for environment variable {0}: {1}")]
    InvalidValue(String, String),

    #[error("Failed to load .env file: {0}")]
    EnvLoadFailed(String),
}