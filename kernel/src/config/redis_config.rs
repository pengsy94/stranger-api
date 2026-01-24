use crate::config::error::ConfigError;
use std::env;

#[derive(Debug, Clone)]
pub struct RedisConfig {
    pub redis_url: String,
}

impl RedisConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let redis_url = env::var("REDIS_URL")
            .unwrap_or_else(|_| "".to_string())
            .parse::<String>()
            .map_err(|_| ConfigError::MissingEnvVar("REDIS_URL".to_string()))?;

        Ok(Self { redis_url })
    }
}
