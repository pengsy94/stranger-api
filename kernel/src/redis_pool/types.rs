use crate::redis_pool::RedisResult;
use crate::redis_pool::service::RedisService;
use anyhow::Result;
use redis::aio::MultiplexedConnection;
use redis::{Client, RedisError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// 常量定义
pub const MATCH_STREAM_KEY: &str = "match:request:stream";
pub const DEAD_LETTER_STREAM_KEY: &str = "match:dead_letter:stream";
pub const CONSUMER_GROUP: &str = "match-consumer-group";
pub const CONSUMER_NAME: &str = "match-consumer-1";
pub const MATCH_POOL_KEY: &str = "match:pool:waiting";
pub const MATCH_STATUS_PREFIX: &str = "match:status:";
pub const MATCH_LOCK_PREFIX: &str = "match:lock:";
pub const MAX_RETRY_COUNT: u8 = 3; // 最大重试次数
pub const MATCH_TIMEOUT_SECS: u64 = 30; // 匹配超时时间（秒）

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

/// Stream消息结构
#[derive(Debug, Clone)]
pub struct StreamMessage {
    pub id: String,
    pub data: String,
}

/// 类型化的消费者结构
#[derive(Debug, Clone)]
pub struct TypedStreamConsumer<T> {
    pub stream_key: String,
    pub consumer_name: String,
    pub batch_size: usize,
    pub block_timeout_ms: usize,
    pub max_retries: usize,
    _marker: std::marker::PhantomData<T>,
}

impl<T> TypedStreamConsumer<T>
where
    T: serde::de::DeserializeOwned + Send + 'static,
{
    pub fn new(
        stream_key: &str,
        consumer_name: &str,
        batch_size: usize,
        block_timeout_ms: usize,
    ) -> Self {
        Self {
            stream_key: stream_key.to_string(),
            consumer_name: consumer_name.to_string(),
            batch_size,
            block_timeout_ms,
            max_retries: 3,
            _marker: std::marker::PhantomData,
        }
    }

    /// 开始消费消息
    pub async fn start_consuming<F, Fut>(&self, processor: F) -> Result<()>
    where
        F: Fn(Vec<(String, T)>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<Vec<String>>> + Send,
    {
        tracing::info!(
            "消费者 {} 开始消费Stream: {}",
            self.consumer_name,
            self.stream_key
        );

        loop {
            match RedisService::consume_messages_generic::<T>(
                &self.stream_key,
                &self.consumer_name,
                self.batch_size,
                self.block_timeout_ms,
            )
            .await
            {
                Ok(messages) => {
                    if !messages.is_empty() {
                        tracing::info!(
                            "消费者 {} 收到 {} 条消息",
                            self.consumer_name,
                            messages.len()
                        );

                        // 处理消息
                        match processor(messages).await {
                            Ok(success_ids) => {
                                // 确认成功处理的消息
                                if !success_ids.is_empty() {
                                    let ids: Vec<&str> =
                                        success_ids.iter().map(|id| id.as_str()).collect();

                                    if let Err(e) =
                                        RedisService::ack_message(&self.stream_key, &ids).await
                                    {
                                        tracing::error!("消息确认失败: {}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::error!("消息处理失败: {}", e);
                                // 这里可以实现重试逻辑或死信队列
                            }
                        }
                    }
                }
                Err(e) => {
                    let err_str = e.to_string();
                    if err_str.contains("timeout")
                        || err_str.contains("TIMEOUT")
                        || err_str.contains("timed out")
                    {
                        // 读取超时是正常的，继续循环
                        continue;
                    }

                    tracing::error!("消费消息失败: {}", e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    /// 获取消费者名称
    pub fn consumer_name(&self) -> &str {
        &self.consumer_name
    }

    /// 获取stream key
    pub fn stream_key(&self) -> &str {
        &self.stream_key
    }
}

/// 生产者发送消息
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MatchRequest {
    pub user_id: String,
    pub game_type: String,
    pub timestamp: i64,
}
