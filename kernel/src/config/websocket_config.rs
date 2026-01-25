use crate::config::error::ConfigError;
use std::env;

#[derive(Debug, Clone)]
pub struct WebSocketConfig {
    pub ws_open: bool,
    pub ws_path: String,
}

impl WebSocketConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let ws_open = env::var("WS_OPEN")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .map_err(|_| ConfigError::MissingEnvVar("WS_OPEN".to_string()))?;

        let ws_path = env::var("WS_PATH")
            .unwrap_or_else(|_| "/ws".to_string())
            .parse::<String>()
            .map_err(|_| ConfigError::MissingEnvVar("WS_PATH".to_string()))?;

        Ok(Self { ws_open, ws_path })
    }
}
