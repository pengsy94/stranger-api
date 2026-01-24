use crate::redis::model::{RedisPool, RedisServiceError};
use std::sync::OnceLock;

pub mod model;
pub mod service;

// 转换为 anyhow::Result 的便捷方法
pub type RedisResult<T> = Result<T, RedisServiceError>;

// 全局 Redis 实例（使用 OnceLock）
static REDIS_POOL: OnceLock<RedisPool> = OnceLock::new();

// 初始化 Redis 连接池
pub async fn init_redis(url: &str) -> RedisResult<()> {
    if url.is_empty() {
        return Ok(());
    }

    let pool = RedisPool::new(url)?;

    // 测试连接
    pool.test_connection().await?;

    // 设置全局池，如果已经初始化则返回错误
    REDIS_POOL
        .set(pool)
        .map_err(|_| RedisServiceError::AlreadyInitialized)?;

    println!("✅ Redis connection pool initialized successfully!");
    Ok(())
}

// 获取全局 Redis 连接池
pub fn get_redis_pool() -> RedisResult<&'static RedisPool> {
    REDIS_POOL
        .get()
        .ok_or_else(|| RedisServiceError::ConnectionError("Redis pool not initialized".to_string()))
}
