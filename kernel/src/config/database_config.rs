use crate::config::error::ConfigError;
use std::env;

/// 数据库配置
#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    pub database_url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout_seconds: u32,
}

impl DatabaseConfig {
    /// 从环境变量创建数据库配置
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "mysql://root:root@localhost:3306/database".to_string())
            .parse::<String>()
            .map_err(|_| ConfigError::MissingEnvVar("DATABASE_URL".to_string()))?;

        let max_connections = env::var("DATABASE_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<u32>()
            .map_err(|e| {
                ConfigError::InvalidValue("DATABASE_MAX_CONNECTIONS".to_string(), e.to_string())
            })?;

        let min_connections = env::var("DATABASE_MIN_CONNECTIONS")
            .unwrap_or_else(|_| "2".to_string())
            .parse::<u32>()
            .map_err(|e| {
                ConfigError::InvalidValue("DATABASE_MIN_CONNECTIONS".to_string(), e.to_string())
            })?;

        let connect_timeout_seconds = env::var("DATABASE_CONNECT_TIMEOUT")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u32>()
            .map_err(|e| {
                ConfigError::InvalidValue("DATABASE_CONNECT_TIMEOUT".to_string(), e.to_string())
            })?;

        Ok(Self {
            database_url,
            max_connections,
            min_connections,
            connect_timeout_seconds,
        })
    }
}
