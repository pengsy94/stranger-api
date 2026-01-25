use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::websocket::models::{ClientMessage, ConnectionManager, ServerMessage};
use axum::{
    extract::{
        Query, State, WebSocketUpgrade,
        ws::{Message, Utf8Bytes, WebSocket},
    },
    response::IntoResponse,
};
use common::request::websocket::WsRequestParams;
use futures_util::{SinkExt, StreamExt};
use kernel::redis::service::RedisService;
use tokio::sync::mpsc;
use tracing::error;

/// WebSocket 升级处理
pub async fn websocket_handler(
    Query(args): Query<WsRequestParams>,
    ws: WebSocketUpgrade,
    State(state): State<Arc<ConnectionManager>>,
) -> impl IntoResponse {
    tracing::info!("新的 WebSocket 连接请求, userKey = {:?}", args);
    ws.on_upgrade(|socket| handle_websocket_connection(socket, state, args))
}

/// 处理 WebSocket 连接
async fn handle_websocket_connection(
    socket: WebSocket,
    state: Arc<ConnectionManager>,
    args: WsRequestParams,
) {
    // 使用浏览器发送过来的userKey作为客户端ID
    let client_id = args.key;

    tracing::info!("处理客户端 {} 的 WebSocket 连接", client_id);

    // 创建与客户端的通信通道
    let (to_client_tx, mut to_client_rx) = mpsc::unbounded_channel();

    // 注册连接
    if !state.register(client_id.clone(), to_client_tx).await {
        tracing::error!("客户端 {} 注册失败", client_id);
        return;
    }

    // 拆分 WebSocket
    let (sender, receiver) = socket.split();

    // 任务1：处理发送给客户端的消息
    let send_task = tokio::spawn({
        let state = state.clone();
        let client_id = client_id.clone();

        async move {
            if let Err(e) = handle_send_task(sender, &mut to_client_rx, &state, &client_id).await {
                tracing::error!("客户端 {} 发送任务错误: {}", client_id, e);
            }
            state.unregister(&client_id).await;
        }
    });

    // 任务2：处理从客户端接收的消息
    let recv_task = tokio::spawn({
        let state = state.clone();
        let client_id = client_id.clone();

        async move {
            if let Err(e) = handle_receive_task(receiver, &state, &client_id).await {
                tracing::error!("客户端 {} 接收任务错误: {}", client_id, e);
            }
        }
    });

    // 等待两个任务完成
    tokio::select! {
        _ = send_task => tracing::info!("客户端 {} 发送任务结束", client_id),
        _ = recv_task => tracing::info!("客户端 {} 接收任务结束", client_id),
    }

    // 确保连接被清理
    state.unregister(&client_id).await;
}

/// 处理发送任务：从通道接收消息并发送给客户端
async fn handle_send_task(
    mut sender: futures_util::stream::SplitSink<WebSocket, Message>,
    to_client_rx: &mut mpsc::UnboundedReceiver<String>,
    state: &ConnectionManager,
    client_id: &str,
) -> Result<(), String> {
    // 发送连接成功消息
    let connected_msg = serde_json::to_string(&ServerMessage::Connected {
        client_id: client_id.to_string(),
        online_count: state.online_count().await,
    })
    .map_err(|e| format!("序列化失败: {}", e))?;

    sender
        .send(Message::Text(Utf8Bytes::from(connected_msg)))
        .await
        .map_err(|e| format!("发送连接消息失败: {}", e))?;

    // 循环处理来自通道的消息
    while let Some(message) = to_client_rx.recv().await {
        sender
            .send(Message::Text(Utf8Bytes::from(message)))
            .await
            .map_err(|e| format!("发送消息失败: {}", e))?;
    }

    Ok(())
}

/// 处理接收任务：从客户端接收消息
async fn handle_receive_task(
    mut receiver: futures_util::stream::SplitStream<WebSocket>,
    state: &ConnectionManager,
    client_id: &str,
) -> Result<(), String> {
    while let Some(result) = receiver.next().await {
        match result {
            Ok(msg) => {
                if let Err(e) = handle_client_message(msg, state, client_id).await {
                    if !e.contains("连接关闭") {
                        tracing::warn!("处理消息失败: {}", e);
                    }
                }
            }
            Err(e) => {
                return Err(format!("接收消息失败: {}", e));
            }
        }
    }

    Ok(())
}

/// 处理客户端消息
async fn handle_client_message(
    msg: Message,
    state: &ConnectionManager,
    client_id: &str,
) -> Result<(), String> {
    match msg {
        Message::Text(text) => {
            tracing::debug!("收到客户端 {} 的消息: {}", client_id, text);

            // 解析客户端消息
            match serde_json::from_str::<ClientMessage>(&text) {
                Ok(client_msg) => handle_parsed_message(client_msg, state, client_id).await,
                Err(e) => Err(format!("消息格式错误: {}", e)),
            }
        }
        Message::Close(_) => {
            tracing::info!("客户端 {} 请求关闭连接", client_id);
            Err("连接关闭".to_string())
        }
        Message::Ping(_data) => {
            tracing::debug!("收到客户端 {} 的 Ping", client_id);
            // 可以在这里发送 Pong 响应，但通常 axum 会自动处理
            Ok(())
        }
        Message::Pong(_) => {
            tracing::debug!("收到客户端 {} 的 Pong", client_id);
            Ok(())
        }
        Message::Binary(_) => Err("不支持二进制消息".to_string()),
    }
}

/// 处理解析后的消息
async fn handle_parsed_message(
    msg: ClientMessage,
    state: &ConnectionManager,
    client_id: &str,
) -> Result<(), String> {
    match msg {
        ClientMessage::Meet {
            user_key,
            age_index,
            sex_index,
            location,
        } => {
            let _list: Vec<String> = RedisService::get_list("MEET_LIST")
                .await
                .unwrap_or_else(|e| {
                    error!("MEET_LIST Error: {}", e);
                    let v: Vec<String> = Vec::new();
                    v
                });

            let meet = serde_json::to_string(&ClientMessage::Meet {
                user_key,
                age_index,
                sex_index,
                location,
            })
            .map_err(|e| format!("序列化失败: {}", e))?;

            // 加入数据到redis队列中
            if let Err(_) = RedisService::lpush_single("MEET_LIST", &meet).await {
                return Err("加入遇见匹配失败！".to_string());
            };

            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let private_msg = serde_json::to_string(&ServerMessage::Private {
                from: client_id.to_string(),
                message: "匹配中...".to_string(),
                timestamp,
            })
            .map_err(|e| format!("序列化失败: {}", e))?;

            state.send_to(&client_id, private_msg).await
        }
        ClientMessage::Private { to, message } => {
            // 检查目标用户是否存在
            if !state.is_online(&to).await {
                return Err(format!("用户 {} 不在线", to));
            }

            // 检查是否是自己
            if to == client_id {
                return Err("不能给自己发送私聊消息".to_string());
            }

            // 发送私聊消息
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            let private_msg = serde_json::to_string(&ServerMessage::Private {
                from: client_id.to_string(),
                message,
                timestamp,
            })
            .map_err(|e| format!("序列化失败: {}", e))?;

            state.send_to(&to, private_msg).await
        }

        ClientMessage::List => {
            // 获取在线用户列表
            let clients = state.list_clients().await;
            let list_msg = serde_json::to_string(&ServerMessage::List { clients })
                .map_err(|e| format!("序列化失败: {}", e))?;

            // 发送给请求者
            state.send_to(client_id, list_msg).await
        }

        ClientMessage::Ping => {
            // 发送 Pong 响应
            let online_count = state.online_count().await;
            let pong_msg = serde_json::to_string(&ServerMessage::Pong { online_count })
                .map_err(|e| format!("序列化失败: {}", e))?;
            println!("pong {}", pong_msg);
            state.send_to(client_id, pong_msg).await
        }

        ClientMessage::Broadcast { message } => {
            // 广播消息
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();

            // 先给自己发送广播确认
            let self_msg = serde_json::to_string(&ServerMessage::Broadcast {
                from: client_id.to_string(),
                message: message.clone(),
                timestamp,
            })
            .map_err(|e| format!("序列化失败: {}", e))?;

            let _ = state.send_to(client_id, self_msg).await;

            // 广播给其他用户
            state.broadcast(client_id, &message, timestamp).await;

            Ok(())
        }
    }
}
