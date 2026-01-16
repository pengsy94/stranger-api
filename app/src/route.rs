use crate::api;
use std::sync::Arc;

use axum::http::StatusCode;
use axum::{
    Router, middleware,
    routing::{get, post},
};
use kernel::config::server_config;
use middleware_fn::request::{logging_middleware, rate_limiter};

pub fn build_router() -> Router {
    let config = server_config();

    let mut router = Router::new();

    // ws服务
    if config.ws_open {
        use crate::websocket::{models::ConnectionManager, set_websocket_api};
        // 创建连接管理器
        let connection_manager = Arc::new(ConnectionManager::new());
        router = router.nest(&config.ws_path, set_websocket_api(connection_manager));
    }

    if config.debug {
        //  测试模块
        router = router.nest("/test", api::case::set_test_api());
    }

    // 添加 API 路由
    router = add_api_routes(router);

    if config.log_enable_oper_log {
        // 整体记录请求
        router = router.layer(middleware::from_fn(logging_middleware));
    }

    router
        .layer(middleware::from_fn(rate_limiter)) // 整体限流
        .fallback(handle_404)
}

fn add_api_routes(router: Router) -> Router {
    router
        .route("/", get(index).post(index))
        .nest("/index", Router::new().route("/", get(index)))
        .nest(
            "/api",
            Router::new().route("/login", post(api::system::login)),
        )
}

async fn index() -> &'static str {
    "Welcome to Axum Api Core!"
}

async fn handle_404() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not found")
}
