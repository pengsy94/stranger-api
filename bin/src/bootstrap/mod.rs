use app::route;
use axum::Router;
use axum::http::Method;
use database::DatabaseManager;
use kernel::config::AppConfig;
use kernel::config::server_config;
use kernel::tasks::manager::SchedulerManager;
use tokio::net::TcpListener;
use tower_http::compression::CompressionLayer;
use tower_http::compression::DefaultPredicate;
use tower_http::compression::Predicate;
use tower_http::compression::predicate::NotForContentType;
use tower_http::cors::Any;
use tower_http::cors::CorsLayer;

pub mod logger;

pub async fn make() -> anyhow::Result<(Router, TcpListener, SchedulerManager)> {
    // 初始化配置（只调用一次）
    AppConfig::init()?;
    // 构建应用
    let (make_service, listener) = build_application().await?;
    // 初始化数据库信息
    DatabaseManager::init().await?;
    // 打印系统信息
    kernel::system::show();
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
