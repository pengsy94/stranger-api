use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{
    sync::{RwLock, mpsc},
    task::JoinHandle,
    time::{self, Duration},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageStruct {
    #[serde(rename = "burnAfterRead")]
    pub burn_after_read: bool,
    pub r#type: u32,
    pub text: String,
}

/// 客户端消息类型
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientMessage {
    /// 匹配命令
    #[serde(rename = "meet")]
    Meet {
        user_key: String,
        age_index: u32,
        sex_index: u32,
        location: String,
    },
    /// 离开某个1对1聊天
    #[serde(rename = "depart")]
    Depart { to: String },
    /// 私聊消息
    #[serde(rename = "private")]
    Private { to: String, message: MessageStruct },
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

/// 等待匹配的用户信息
#[derive(Debug, Clone)]
pub struct WaitingUser {
    pub client_id: String,
    pub user_key: String,
    pub age_index: u32,
    pub sex_index: u32,
    pub location: String,
    pub join_time: u64,
}

/// 服务器消息类型
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerMessage {
    /// 连接成功
    #[serde(rename = "connected")]
    Connected {
        client_id: String,
        online_count: usize,
    },
    #[serde(rename = "meet_loading")]
    MeetLoading { message: String },
    #[serde(rename = "meet")]
    Meet { user_key: String },
    #[serde(rename = "meet_failed")]
    MeetFailed { message: String },
    #[serde(rename = "meet_success")]
    MeetSuccess {
        to: String,
        message: String,
        age: u32,
        sex: u32,
        location: String,
    },
    /// 离开某个1对1聊天
    #[serde(rename = "depart")]
    Depart { from: String },
    /// 私聊消息
    #[serde(rename = "private")]
    Private {
        from: String,
        message: MessageStruct,
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
    Pong { online_count: usize },
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
    /// 等待匹配的用户队列
    waiting_queue: Arc<RwLock<VecDeque<WaitingUser>>>,
    /// 存储用户的匹配超时定时器
    match_timers: Arc<RwLock<HashMap<String, JoinHandle<()>>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            waiting_queue: Arc::new(RwLock::new(VecDeque::new())),
            match_timers: Arc::new(RwLock::new(HashMap::new())),
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

    /// 添加用户到匹配队列
    pub async fn add_to_waiting_queue(
        &self,
        client_id: String,
        user_key: String,
        age_index: u32,
        sex_index: u32,
        location: String,
    ) {
        // 先检查用户是否已经在队列中
        let waiting_queue_read = self.waiting_queue.read().await;
        let already_in_queue = waiting_queue_read
            .iter()
            .any(|user| user.client_id == client_id);

        if already_in_queue {
            tracing::info!("用户 {} 已经在匹配队列中，无需重复添加", client_id);
            return;
        }
        drop(waiting_queue_read);

        let waiting_user = WaitingUser {
            client_id: client_id.clone(),
            user_key,
            age_index,
            sex_index,
            location,
            join_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        let mut waiting_queue = self.waiting_queue.write().await;
        waiting_queue.push_back(waiting_user);
        tracing::info!("用户 {} 加入匹配队列", client_id);
        drop(waiting_queue);

        // 设置10秒超时定时器
        let manager_clone = self.clone();
        let client_id_clone = client_id.clone();

        let timer = tokio::spawn(async move {
            time::sleep(Duration::from_secs(10)).await;

            // 检查用户是否还在队列中
            let waiting_queue_read = manager_clone.waiting_queue.read().await;
            let still_waiting = waiting_queue_read
                .iter()
                .any(|user| user.client_id == client_id_clone);
            drop(waiting_queue_read);

            if still_waiting {
                // 从队列中移除用户
                manager_clone
                    .remove_from_waiting_queue(&client_id_clone)
                    .await;

                // 发送超时消息给用户
                let timeout_msg = serde_json::to_string(&ServerMessage::MeetFailed {
                    message: "匹配超时，请重试".to_string(),
                })
                .unwrap_or_else(|_| {
                    "{\"type\":\"error\",\"message\":\"消息序列化失败\"}".to_string()
                });

                let _ = manager_clone.send_to(&client_id_clone, timeout_msg).await;
                tracing::info!("用户 {} 匹配超时，已退出匹配", client_id_clone);
            }

            // 移除定时器
            let mut timers = manager_clone.match_timers.write().await;
            timers.remove(&client_id_clone);
        });

        // 存储定时器
        let mut timers = self.match_timers.write().await;
        timers.insert(client_id.clone(), timer);
    }

    /// 从匹配队列中移除用户
    pub async fn remove_from_waiting_queue(&self, client_id: &str) {
        let mut waiting_queue = self.waiting_queue.write().await;
        waiting_queue.retain(|user| user.client_id != client_id);
        tracing::info!("用户 {} 从匹配队列中移除", client_id);
    }

    /// 执行匹配算法
    pub async fn match_users(&self) -> Option<(WaitingUser, WaitingUser)> {
        let mut waiting_queue = self.waiting_queue.write().await;
        if waiting_queue.len() < 2 {
            return None;
        }

        // 优先匹配不同性别的用户
        let mut matched_indices = None;

        // 遍历队列，寻找不同性别的匹配
        for (i, user1) in waiting_queue.iter().enumerate() {
            for (j, user2) in waiting_queue.iter().enumerate() {
                if i < j && user1.sex_index != user2.sex_index {
                    matched_indices = Some((i, j));
                    break;
                }
            }
            if matched_indices.is_some() {
                break;
            }
        }

        // 如果没有找到不同性别的匹配，就匹配最先加入队列的两个用户
        if matched_indices.is_none() {
            matched_indices = Some((0, 1));
        }

        if let Some((i, j)) = matched_indices {
            // 确保i < j
            let (first_idx, second_idx) = if i < j { (i, j) } else { (j, i) };

            // 获取匹配的用户
            let user2 = waiting_queue.remove(second_idx).unwrap();
            let user1 = waiting_queue.remove(first_idx).unwrap();

            // 取消两个用户的匹配定时器
            let mut timers = self.match_timers.write().await;
            if let Some(timer) = timers.remove(&user1.client_id) {
                timer.abort();
            }
            if let Some(timer) = timers.remove(&user2.client_id) {
                timer.abort();
            }
            drop(timers);

            tracing::info!("成功匹配用户 {} 和 {}", user1.client_id, user2.client_id);
            return Some((user1, user2));
        }

        None
    }

    /// 发送匹配结果给双方用户
    pub async fn notify_match_result(
        &self,
        user1: &WaitingUser,
        user2: &WaitingUser,
    ) -> Result<(), String> {
        // let timestamp = SystemTime::now()
        //     .duration_since(UNIX_EPOCH)
        //     .unwrap()
        //     .as_secs();

        // 给用户1发送匹配成功消息
        let user1_msg = serde_json::to_string(&ServerMessage::Meet {
            user_key: user2.user_key.clone(),
        })
        .map_err(|e| format!("序列化失败: {}", e))?;

        // 给用户2发送匹配成功消息
        let user2_msg = serde_json::to_string(&ServerMessage::Meet {
            user_key: user1.user_key.clone(),
        })
        .map_err(|e| format!("序列化失败: {}", e))?;

        // 发送匹配成功消息
        let _ = self.send_to(&user1.client_id, user1_msg).await;
        let _ = self.send_to(&user2.client_id, user2_msg).await;

        // 发送系统消息提示匹配成功
        let system_msg1 = serde_json::to_string(&ServerMessage::MeetSuccess {
            to: user2.user_key.clone(),
            age: user2.age_index,
            sex: user2.sex_index,
            location: user2.location.clone(),
            message: "匹配成功，开始聊天吧！".to_string(),
        })
        .map_err(|e| format!("序列化失败: {}", e))?;

        let system_msg2 = serde_json::to_string(&ServerMessage::MeetSuccess {
            to: user1.user_key.clone(),
            age: user1.age_index,
            sex: user1.sex_index,
            location: user1.location.clone(),
            message: "匹配成功，开始聊天吧！".to_string(),
        })
        .map_err(|e| format!("序列化失败: {}", e))?;

        let _ = self.send_to(&user1.client_id, system_msg1).await;
        let _ = self.send_to(&user2.client_id, system_msg2).await;

        Ok(())
    }
}
