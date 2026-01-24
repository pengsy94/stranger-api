mod database_config;
pub mod error;
mod redis_config;
mod server_config;

use crate::config::{
    database_config::DatabaseConfig, redis_config::RedisConfig, server_config::ServerConfig,
};
use dotenvy::dotenv;
use error::ConfigError;
use std::sync::OnceLock;

/// 全局配置单例
static CONFIG: OnceLock<AppConfig> = OnceLock::new();

/// 应用配置
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub server: ServerConfig,
    pub redis: RedisConfig,
}

impl AppConfig {
    /// 初始化配置（应用启动时调用一次）
    pub fn init() -> Result<(), ConfigError> {
        // 如果已经初始化，返回错误
        if CONFIG.get().is_some() {
            return Err(ConfigError::AlreadyInitialized);
        }

        // 加载 .env 文件
        dotenv().map_err(|e| ConfigError::EnvLoadFailed(e.to_string()))?;

        // 从环境变量创建配置
        let config = Self::from_env()?;

        // 设置全局单例
        CONFIG
            .set(config)
            .map_err(|_| ConfigError::AlreadyInitialized)
    }

    /// 从环境变量创建配置
    fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            server: ServerConfig::from_env()?,
            database: DatabaseConfig::from_env()?,
            redis: RedisConfig::from_env()?,
        })
    }

    /// 获取全局配置(初始化后使用)
    pub fn global() -> &'static AppConfig {
        CONFIG
            .get()
            .expect("Configuration not initialized. Call AppConfig::init() first")
    }

    /// 安全获取配置（不会 panic）
    pub fn try_global() -> Option<&'static AppConfig> {
        CONFIG.get()
    }
}

/// 便捷函数：获取服务器配置
pub fn server_config() -> &'static ServerConfig {
    &AppConfig::global().server
}

/// 便捷函数：获取数据库配置
pub fn database_config() -> &'static DatabaseConfig {
    &AppConfig::global().database
}

/// 便捷函数：获取redis配置
pub fn redis_config() -> &'static RedisConfig {
    &AppConfig::global().redis
}
