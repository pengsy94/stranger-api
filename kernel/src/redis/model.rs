use redis::{Client, RedisError};
use redis::aio::MultiplexedConnection;
use thiserror::Error;
use crate::redis::RedisResult;

// Redis 操作专用错误类型（不实现 IntoResponse）
#[derive(Error, Debug)]
pub enum RedisServiceError {
    #[error("Redis connection error: {0}")]
    ConnectionError(String),

    #[error("Redis operation error: {0}")]
    OperationError(#[from] RedisError),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Key not found: {0}")]
    NotFound(String),

    #[error("Redis pool already initialized")]
    AlreadyInitialized,

    #[error("Redis pool not initialized")]
    NotInitialized,
}

// 简单的 Redis 连接池结构
#[derive(Debug, Clone)]
pub struct RedisPool {
    client: Client,
}

impl RedisPool {
    // 创建新连接池
    pub fn new(url: &str) -> RedisResult<Self> {
        let client =
            Client::open(url).map_err(|e| RedisServiceError::ConnectionError(e.to_string()))?;
        Ok(Self { client })
    }

    // 获取异步连接
    pub async fn get_connection(&self) -> RedisResult<MultiplexedConnection> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| RedisServiceError::ConnectionError(e.to_string()))
    }

    // 测试连接
    pub async fn test_connection(&self) -> RedisResult<()> {
        let mut conn = self.get_connection().await?;
        let result: Result<String, RedisError> = redis::cmd("PING").query_async(&mut conn).await;

        result
            .map(|_| ()) // 忽略返回值，只关心是否成功
            .map_err(|e| RedisServiceError::ConnectionError(format!("PING failed: {}", e)))
    }
}
