use crate::redis::model::RedisServiceError;
use crate::redis::{RedisResult, get_redis_pool};
use redis::{AsyncCommands, RedisError, aio::MultiplexedConnection};

// Redis 服务
pub struct RedisService;

impl RedisService {
    // 获取 Redis 连接
    async fn get_conn() -> RedisResult<MultiplexedConnection> {
        let pool = get_redis_pool()?;
        pool.get_connection().await
    }

    // 设置值
    pub async fn set(key: &str, value: &str, ttl_seconds: Option<u64>) -> RedisResult<()> {
        let mut conn = Self::get_conn().await?;

        match ttl_seconds {
            Some(ttl) => conn.set_ex(key, value, ttl).await,
            None => conn.set(key, value).await,
        }
        .map_err(RedisServiceError::OperationError)
    }

    // 获取值
    pub async fn get(key: &str) -> RedisResult<Option<String>> {
        let mut conn = Self::get_conn().await?;
        conn.get(key)
            .await
            .map_err(RedisServiceError::OperationError)
    }

    // 删除键
    pub async fn delete(key: &str) -> RedisResult<()> {
        let mut conn = Self::get_conn().await?;
        conn.del(key)
            .await
            .map_err(RedisServiceError::OperationError)
    }

    // 递增计数器
    pub async fn incr(key: &str, increment: i64) -> RedisResult<i64> {
        let mut conn = Self::get_conn().await?;
        conn.incr(key, increment)
            .await
            .map_err(RedisServiceError::OperationError)
    }

    // 检查键是否存在
    pub async fn exists(key: &str) -> RedisResult<bool> {
        let mut conn = Self::get_conn().await?;
        conn.exists(key)
            .await
            .map_err(RedisServiceError::OperationError)
    }

    // 获取 TTL
    pub async fn ttl(key: &str) -> RedisResult<Option<i64>> {
        let mut conn = Self::get_conn().await?;
        let result = conn
            .ttl(key)
            .await
            .map_err(RedisServiceError::OperationError)?;

        match result {
            -2 => Ok(None),     // 键不存在
            -1 => Ok(Some(-1)), // 永久有效
            ttl => Ok(Some(ttl)),
        }
    }

    // 健康检查
    pub async fn ping() -> RedisResult<String> {
        let mut conn = Self::get_conn().await?;
        redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(RedisServiceError::OperationError)
    }

    // 推入单个元素
    pub async fn lpush_single(key: &str, value: &str) -> RedisResult<usize> {
        let mut conn = Self::get_conn().await?;

        // 直接使用 AsyncCommands trait 的方法
        conn.lpush(key, value)
            .await
            .map_err(RedisServiceError::OperationError)
    }

    // 从左侧推入元素
    pub async fn lpush(key: &str, values: &[&str]) -> RedisResult<usize> {
        let mut conn = Self::get_conn().await?;

        // 使用 cmd 执行 LPUSH 命令
        let result: Result<usize, RedisError> = redis::cmd("LPUSH")
            .arg(key)
            .arg(values)
            .query_async(&mut conn)
            .await;

        result.map_err(RedisServiceError::OperationError)
    }

    // 从右侧推入元素
    pub async fn rpush(key: &str, values: &[&str]) -> RedisResult<usize> {
        let mut conn = Self::get_conn().await?;

        let result: Result<usize, RedisError> = redis::cmd("RPUSH")
            .arg(key)
            .arg(values)
            .query_async(&mut conn)
            .await;

        result.map_err(RedisServiceError::OperationError)
    }

    // 从左侧弹出元素
    pub async fn lpop(key: &str) -> RedisResult<Option<String>> {
        let mut conn = Self::get_conn().await?;

        let result: Result<Option<String>, RedisError> =
            redis::cmd("LPOP").arg(key).query_async(&mut conn).await;

        result.map_err(RedisServiceError::OperationError)
    }

    // 从右侧弹出元素
    pub async fn rpop(key: &str) -> RedisResult<Option<String>> {
        let mut conn = Self::get_conn().await?;

        let result: Result<Option<String>, RedisError> =
            redis::cmd("RPOP").arg(key).query_async(&mut conn).await;

        result.map_err(RedisServiceError::OperationError)
    }

    // 获取列表长度
    pub async fn llen(key: &str) -> RedisResult<usize> {
        let mut conn = Self::get_conn().await?;

        let result: Result<usize, RedisError> =
            redis::cmd("LLEN").arg(key).query_async(&mut conn).await;

        result.map_err(RedisServiceError::OperationError)
    }

    // 获取列表范围
    pub async fn lrange(key: &str, start: isize, stop: isize) -> RedisResult<Vec<String>> {
        let mut conn = Self::get_conn().await?;

        let result: Result<Vec<String>, RedisError> = redis::cmd("LRANGE")
            .arg(key)
            .arg(start)
            .arg(stop)
            .query_async(&mut conn)
            .await;

        result.map_err(RedisServiceError::OperationError)
    }

    // 获取整个列表
    pub async fn get_list(key: &str) -> RedisResult<Vec<String>> {
        Self::lrange(key, 0, -1).await
    }

    // 通过索引获取元素
    pub async fn lindex(key: &str, index: isize) -> RedisResult<Option<String>> {
        let mut conn = Self::get_conn().await?;

        let result: Result<Option<String>, RedisError> = redis::cmd("LINDEX")
            .arg(key)
            .arg(index)
            .query_async(&mut conn)
            .await;

        result.map_err(RedisServiceError::OperationError)
    }

    // 修剪列表
    pub async fn ltrim(key: &str, start: isize, stop: isize) -> RedisResult<()> {
        let mut conn = Self::get_conn().await?;

        let result: Result<(), RedisError> = redis::cmd("LTRIM")
            .arg(key)
            .arg(start)
            .arg(stop)
            .query_async(&mut conn)
            .await;

        result.map_err(RedisServiceError::OperationError)
    }

    // 移除元素
    pub async fn lrem(key: &str, count: isize, value: &str) -> RedisResult<usize> {
        let mut conn = Self::get_conn().await?;

        let result: Result<usize, RedisError> = redis::cmd("LREM")
            .arg(key)
            .arg(count)
            .arg(value)
            .query_async(&mut conn)
            .await;

        result.map_err(RedisServiceError::OperationError)
    }

    // 设置指定索引位置的元素
    pub async fn lset(key: &str, index: isize, value: &str) -> RedisResult<()> {
        let mut conn = Self::get_conn().await?;

        let result: Result<(), RedisError> = redis::cmd("LSET")
            .arg(key)
            .arg(index)
            .arg(value)
            .query_async(&mut conn)
            .await;

        result.map_err(RedisServiceError::OperationError)
    }

    // 阻塞式左侧弹出（BLPOP）
    pub async fn blpop(keys: &[&str], timeout: usize) -> RedisResult<Option<(String, String)>> {
        let mut conn = Self::get_conn().await?;

        let result: Result<Option<(String, String)>, RedisError> = redis::cmd("BLPOP")
            .arg(keys)
            .arg(timeout)
            .query_async(&mut conn)
            .await;

        result.map_err(RedisServiceError::OperationError)
    }

    // 阻塞式右侧弹出（BRPOP）
    pub async fn brpop(keys: &[&str], timeout: usize) -> RedisResult<Option<(String, String)>> {
        let mut conn = Self::get_conn().await?;

        let result: Result<Option<(String, String)>, RedisError> = redis::cmd("BRPOP")
            .arg(keys)
            .arg(timeout)
            .query_async(&mut conn)
            .await;

        result.map_err(RedisServiceError::OperationError)
    }

    // RPOPLPUSH - 原子操作，从源列表弹出并推入目标列表
    pub async fn rpoplpush(source: &str, destination: &str) -> RedisResult<Option<String>> {
        let mut conn = Self::get_conn().await?;

        let result: Result<Option<String>, RedisError> = redis::cmd("RPOPLPUSH")
            .arg(source)
            .arg(destination)
            .query_async(&mut conn)
            .await;

        result.map_err(RedisServiceError::OperationError)
    }
}
