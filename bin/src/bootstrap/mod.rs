use app::route;
use axum::{Router, http::Method};
use kernel::{
    config::{AppConfig, database_config, redis_config, server_config},
    tasks::manager::SchedulerManager,
};
use std::process;
use tokio::net::TcpListener;
use tower_http::{
    compression::{CompressionLayer, DefaultPredicate, Predicate, predicate::NotForContentType},
    cors::{Any, CorsLayer},
};

pub mod logger;

pub async fn make() -> anyhow::Result<(Router, TcpListener, SchedulerManager)> {
    // 初始化配置（只调用一次）
    if let Err(e) = AppConfig::init() {
        eprintln!("❌ Failed to initialize app config: {}", e);
        process::exit(1);
    };

    // 构建应用
    let (make_service, listener) = build_application().await?;

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
    }

    // 创建调度器管理器
    let scheduler_manager = SchedulerManager::new();
    // 启动定时任务
    scheduler_manager.start().await.unwrap();

    Ok((make_service, listener, scheduler_manager))
}

async fn build_application() -> anyhow::Result<(Router, TcpListener)> {
    let config = server_config();

    let app = route::build_router();
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
