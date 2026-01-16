use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::{RwLock, mpsc};

/// 客户端消息类型
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// 私聊消息
    #[serde(rename = "private")]
    Private { to: String, message: String },
    /// 获取在线用户列表
    #[serde(rename = "list")]
    List,
    /// 心跳包
    #[serde(rename = "ping")]
    Ping,
    /// 广播消息
    #[serde(rename = "broadcast")]
    Broadcast { message: String },
}

/// 客户端信息
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClientInfo {
    pub id: String,
    pub connected_at: u64,
}

/// 服务器消息类型
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    /// 连接成功
    #[serde(rename = "connected")]
    Connected {
        client_id: String,
        online_count: usize,
    },
    /// 私聊消息
    #[serde(rename = "private")]
    Private {
        from: String,
        message: String,
        timestamp: u64,
    },
    /// 在线用户列表
    #[serde(rename = "list")]
    List { clients: Vec<ClientInfo> },
    /// 系统消息
    #[serde(rename = "system")]
    System { message: String },
    /// 错误消息
    #[serde(rename = "error")]
    Error { message: String },
    /// 心跳响应
    #[serde(rename = "pong")]
    Pong,
    /// 广播消息
    #[serde(rename = "broadcast")]
    Broadcast {
        from: String,
        message: String,
        timestamp: u64,
    },
}

/// 客户端连接
struct ClientConnection {
    /// 发送消息的通道
    sender: mpsc::UnboundedSender<String>,
    /// 连接时间
    connected_at: u64,
}

/// 连接管理器
#[derive(Clone)]
pub struct ConnectionManager {
    /// 存储所有连接
    connections: Arc<RwLock<HashMap<String, ClientConnection>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 注册新连接
    pub async fn register(&self, client_id: String, sender: mpsc::UnboundedSender<String>) -> bool {
        let connection = ClientConnection {
            sender,
            connected_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let mut connections = self.connections.write().await;
        if connections.contains_key(&client_id) {
            return false;
        }

        connections.insert(client_id.clone(), connection);
        tracing::info!("客户端 {} 已连接", client_id);

        true
    }

    /// 注销连接
    pub async fn unregister(&self, client_id: &str) {
        let mut connections = self.connections.write().await;
        if connections.remove(client_id).is_some() {
            tracing::info!("客户端 {} 已断开连接", client_id);
        }
    }

    /// 向指定客户端发送消息
    pub async fn send_to(&self, target_id: &str, message: String) -> Result<(), String> {
        let connections = self.connections.read().await;

        match connections.get(target_id) {
            Some(connection) => connection
                .sender
                .send(message)
                .map_err(|e| format!("发送失败: {}", e)),
            None => Err(format!("用户 {} 不在线", target_id)),
        }
    }

    /// 广播消息给所有用户（除了发送者）
    pub async fn broadcast(&self, from: &str, message: &str, timestamp: u64) {
        let connections = self.connections.read().await;

        for (client_id, connection) in connections.iter() {
            if client_id == from {
                continue; // 不发送给自己
            }

            let broadcast_msg = serde_json::to_string(&ServerMessage::Broadcast {
                from: from.to_string(),
                message: message.to_string(),
                timestamp,
            })
                .unwrap_or_else(|_| "{\"type\":\"error\",\"message\":\"消息序列化失败\"}".to_string());

            let _ = connection.sender.send(broadcast_msg);
        }
    }

    /// 获取在线用户列表
    pub async fn list_clients(&self) -> Vec<ClientInfo> {
        let connections = self.connections.read().await;

        connections
            .iter()
            .map(|(id, conn)| ClientInfo {
                id: id.clone(),
                connected_at: conn.connected_at,
            })
            .collect()
    }

    /// 获取在线用户数量
    pub async fn online_count(&self) -> usize {
        self.connections.read().await.len()
    }

    /// 检查用户是否在线
    pub async fn is_online(&self, client_id: &str) -> bool {
        self.connections.read().await.contains_key(client_id)
    }
}
