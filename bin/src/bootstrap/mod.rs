use anyhow::Result;
use app::route;
use app::websocket::types::ConnectionManager;
use axum::{Router, http::Method};
use kernel::redis_pool::types::{MATCH_STREAM_KEY, MatchRequest, TypedStreamConsumer};
use kernel::{
    config::{AppConfig, database_config, redis_config, server_config},
    tasks::manager::SchedulerManager,
};
use std::process;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::{
    compression::{CompressionLayer, DefaultPredicate, Predicate, predicate::NotForContentType},
    cors::{Any, CorsLayer},
};

pub mod logger;

pub async fn make() -> Result<(Router, TcpListener, SchedulerManager)> {
    // 初始化配置（只调用一次）
    if let Err(e) = AppConfig::init() {
        eprintln!("❌ Failed to initialize app config: {}", e);
        process::exit(1);
    };

    // 创建ws连接管理器
    let connection_manager = Arc::new(ConnectionManager::new());
    let scheduler_cm = connection_manager.clone();

    // 构建应用
    let (make_service, listener) = match build_application(connection_manager).await {
        Ok((make_service, listener)) => (make_service, listener),
        Err(e) => {
            eprintln!("❌ Failed to initialize build Application: {}", e);
            process::exit(1);
        }
    };

    // 打印系统信息
    kernel::system::show();

    let config = database_config();
    if !config.database_url.is_empty() {
        use database::DatabaseManager;
        // 初始化数据库信息
        if let Err(e) = DatabaseManager::init().await {
            eprintln!("❌ Failed to initialize Database: {}", e);
            eprintln!(
                "💡 Make sure Database is running at: {}",
                config.database_url
            );
            process::exit(1);
        };
    }

    let config = redis_config();
    if !config.redis_url.is_empty() {
        use kernel::redis_pool::init_redis;
        use kernel::redis_pool::service::RedisService;
        // 初始化 Redis 连接池
        if let Err(e) = init_redis(&config.redis_url).await {
            eprintln!("❌ Failed to initialize Redis: {}", e);
            eprintln!("💡 Make sure Redis is running at: {}", config.redis_url);
            process::exit(1);
        }

        // 初始化Redis Stream和消费组
        if let Err(e) = RedisService::init_redis_stream().await {
            eprintln!(
                "❌ Failed to initialize the Stream and the consumption group: {}",
                e
            );
            process::exit(1);
        }

        // 创建消费者
        let consumer = TypedStreamConsumer::<MatchRequest>::new(
            MATCH_STREAM_KEY,
            "worker-1", // 消费者名称，每个消费者应该唯一
            1,          // 每次读取1条消息
            5000,       // 阻塞超时5秒
        );

        // 启动消费者（通常在一个独立的tokio任务中）
        tokio::spawn(async move {
            let ws_manager = scheduler_cm.clone();
            consumer
                .start_consuming(move |messages| {
                    let ws_manager = ws_manager.clone();
                    async move {
                        let mut success_ids = Vec::new();

                        for (msg_id, data) in messages {
                            println!("处理消息 ID: {}, 数据: {:?}", msg_id, data);

                            let _send = ws_manager.send_to(&"xiaofeng", "".to_string()).await;
                            // 业务处理逻辑
                            // ...
                            success_ids.push(msg_id);
                        }
                        Ok(success_ids)
                    }
                })
                .await
        });

        let request = MatchRequest {
            user_id: "user_123".to_string(),
            game_type: "pvp".to_string(),
            timestamp: chrono::Utc::now().timestamp(),
        };
        let _message_id = RedisService::add_message_to_stream(MATCH_STREAM_KEY, &request).await?;
    }

    // 创建调度器管理器
    let scheduler_manager = SchedulerManager::new();
    // 启动定时任务
    scheduler_manager.start().await.unwrap();

    Ok((make_service, listener, scheduler_manager))
}

async fn build_application(
    connection_manager: Arc<ConnectionManager>,
) -> Result<(Router, TcpListener)> {
    let config = server_config();

    let app = route::build_router(connection_manager);
    let app = match &config.content_gzip {
        true => {
            //  开启压缩后 SSE 数据无法返回  text/event-stream 单独处理不压缩
            let predicate =
                DefaultPredicate::new().and(NotForContentType::new("text/event-stream"));
            app.layer(CompressionLayer::new().compress_when(predicate))
        }
        false => app,
    };

    // 添加cors跨越
    let make_service = app.layer(setup_cors());

    let addr = format!("{}:{}", config.host, config.port);
    let listener = TcpListener::bind(addr).await?;
    Ok((make_service, listener))
}

fn setup_cors() -> CorsLayer {
    let methods = vec![Method::GET, Method::POST, Method::HEAD, Method::OPTIONS];

    CorsLayer::new()
        .allow_methods(methods)
        .allow_origin(Any)
        .allow_headers(Any)
}
