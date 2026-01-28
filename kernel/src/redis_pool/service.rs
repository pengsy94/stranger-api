use crate::redis_pool::types::{CONSUMER_GROUP, DEAD_LETTER_STREAM_KEY, MATCH_STREAM_KEY};
use crate::redis_pool::{RedisResult, get_redis_pool};
use anyhow::{Context, Result};
use redis::{Cmd, Value, aio::MultiplexedConnection};
use serde::Serialize;

// Redis 服务
pub struct RedisService;

impl RedisService {
    // 获取 Redis 连接
    async fn get_conn() -> RedisResult<MultiplexedConnection> {
        let pool = get_redis_pool()?;
        pool.get_connection().await
    }

    /// 初始化Stream和消费组
    pub async fn init_redis_stream() -> Result<()> {
        // 初始化主匹配流
        Self::init_consumer_group(MATCH_STREAM_KEY).await?;
        // 初始化死信流
        Self::init_consumer_group(DEAD_LETTER_STREAM_KEY).await?;

        Ok(())
    }

    /// 初始化消费组
    async fn init_consumer_group(stream_key: &str) -> Result<()> {
        let mut conn = Self::get_conn().await?;

        // 检查消费组是否存在
        match redis::cmd("XGROUP")
            .arg("CREATE")
            .arg(stream_key)
            .arg(CONSUMER_GROUP)
            .arg("$")
            .arg("MKSTREAM")
            .query_async::<String>(&mut conn)
            .await
        {
            Ok(_) => {
                tracing::info!("创建消费组 {}:{} 成功", stream_key, CONSUMER_GROUP);
                Ok(())
            }
            Err(e) => {
                // 如果消费组已存在，会返回 BUSYGROUP 错误，这是正常的
                let err_msg = e.to_string();
                if err_msg.contains("BUSYGROUP") {
                    tracing::info!("消费组 {}:{} 已存在", stream_key, CONSUMER_GROUP);
                    Ok(())
                } else {
                    // 其他错误需要处理
                    Err(anyhow::anyhow!("创建消费组失败: {}", e))
                }
            }
        }
    }

    /// 向Stream添加消息（生产者方法）
    pub async fn add_message_to_stream<T: Serialize>(
        stream_key: &str,
        message: &T,
    ) -> Result<String> {
        let mut conn = Self::get_conn().await?;

        // 将消息序列化为JSON字符串
        let message_json = serde_json::to_string(message)?;

        // 使用 XADD 命令
        let result: String = Cmd::new()
            .arg("XADD")
            .arg(stream_key)
            .arg("*") // 使用 * 让Redis自动生成ID
            .arg("data")
            .arg(&message_json)
            .query_async(&mut conn)
            .await
            .with_context(|| format!("向Stream {} 添加消息失败", stream_key))?;
        println!("向Stream {} 添加消息成功，ID: {}", stream_key, result);

        Ok(result)
    }

    /// 批量添加消息
    pub async fn add_messages_to_stream<T: Serialize>(
        stream_key: &str,
        messages: &[T],
    ) -> Result<Vec<String>> {
        let mut results = Vec::new();

        for message in messages {
            let message_id = Self::add_message_to_stream(stream_key, message).await?;
            results.push(message_id);
        }

        Ok(results)
    }

    /// 消费消息（消费者方法）
    pub async fn consume_messages(
        stream_key: &str,
        consumer_name: &str,
        count: usize,
        block_ms: usize,
    ) -> Result<Vec<(String, String)>> {
        let mut conn = Self::get_conn().await?;

        // 使用 XREADGROUP 读取消息
        // > 表示只读取未ack的消息
        let result: Vec<(String, Vec<(String, Vec<(String, String)>)>)> = redis::cmd("XREADGROUP")
            .arg("GROUP")
            .arg(CONSUMER_GROUP)
            .arg(consumer_name)
            .arg("COUNT")
            .arg(count)
            .arg("BLOCK")
            .arg(block_ms)
            .arg("STREAMS")
            .arg(stream_key)
            .arg(">")
            .query_async(&mut conn)
            .await?;

        let mut messages = Vec::new();

        for (_stream_key, message_list) in result {
            for (message_id, fields) in message_list {
                // 查找 data 字段
                for (field_name, field_value) in fields {
                    if field_name == "data" {
                        messages.push((message_id, field_value));
                        break;
                    }
                }
            }
        }

        Ok(messages)
    }

    /// 消费消息并自动反序列化
    pub async fn consume_messages_generic<T: serde::de::DeserializeOwned>(
        stream_key: &str,
        consumer_name: &str,
        count: usize,
        block_ms: usize,
    ) -> Result<Vec<(String, T)>> {
        let raw_messages =
            Self::consume_messages(stream_key, consumer_name, count, block_ms).await?;

        let mut messages = Vec::new();

        for (message_id, data_json) in raw_messages {
            match serde_json::from_str::<T>(&data_json) {
                Ok(data) => {
                    messages.push((message_id, data));
                }
                Err(e) => {
                    tracing::error!(
                        "解析消息失败: {}, 消息ID: {}, 原始数据: {}",
                        e,
                        message_id,
                        data_json
                    );
                }
            }
        }

        Ok(messages)
    }

    /// 确认消息已处理
    pub async fn ack_message(stream_key: &str, message_ids: &[&str]) -> Result<usize> {
        let mut conn = Self::get_conn().await?;

        let acked_count: usize = redis::cmd("XACK")
            .arg(stream_key)
            .arg(CONSUMER_GROUP)
            .arg(message_ids)
            .query_async(&mut conn)
            .await?;

        tracing::debug!("已确认 {} 条消息", acked_count);
        Ok(acked_count)
    }

    /// 查看待处理消息数量
    pub async fn get_pending_count(stream_key: &str) -> Result<usize> {
        let mut conn = Self::get_conn().await?;

        let pending_info: Vec<Value> = redis::cmd("XPENDING")
            .arg(stream_key)
            .arg(CONSUMER_GROUP)
            .query_async(&mut conn)
            .await?;

        // XPENDING 返回 [total_pending, start_id, end_id, [consumer1, count1], ...]
        if let Some(redis::Value::Int(count)) = pending_info.get(0) {
            Ok(*count as usize)
        } else {
            Ok(0)
        }
    }

    /// 查看消费者信息
    pub async fn get_consumer_info(stream_key: &str) -> Result<Vec<(String, usize, u64, usize)>> {
        let mut conn = Self::get_conn().await?;

        // XINFO CONSUMERS 返回类型
        let result: Vec<(String, usize, u64, usize)> = redis::cmd("XINFO")
            .arg("CONSUMERS")
            .arg(stream_key)
            .arg(CONSUMER_GROUP)
            .query_async(&mut conn)
            .await?;

        Ok(result)
    }

    /// 重新投递失败的消息
    pub async fn claim_pending_messages(
        stream_key: &str,
        consumer_name: &str,
        min_idle_time_ms: usize,
        count: usize,
    ) -> Result<Vec<(String, String)>> {
        let mut conn = Self::get_conn().await?;

        let result: Vec<Vec<Value>> = redis::cmd("XCLAIM")
            .arg(stream_key)
            .arg(CONSUMER_GROUP)
            .arg(consumer_name)
            .arg(min_idle_time_ms)
            .arg("IDLE")
            .arg("0")
            .arg("RETRYCOUNT")
            .arg("0")
            .arg("FORCE")
            .arg("COUNT")
            .arg(count)
            .arg("JUSTID")
            .query_async(&mut conn)
            .await?;

        // 解析结果
        let mut messages = Vec::new();

        for item in result {
            if item.len() >= 2 {
                if let (Value::BulkString(id_bytes), Value::BulkString(data_bytes)) =
                    (&item[0], &item[1])
                {
                    let message_id = String::from_utf8_lossy(id_bytes).to_string();
                    let data = String::from_utf8_lossy(data_bytes).to_string();
                    messages.push((message_id, data));
                }
            }
        }

        Ok(messages)
    }
}
