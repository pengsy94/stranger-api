use axum::{
    Json,
    extract::{OriginalUri, Request},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;
use std::time::Instant;
use tokio::time::sleep;
use tracing::info;

use serde_json::json;

pub async fn logging_middleware(
    OriginalUri(original_uri): OriginalUri, // 原始地址
    request: Request,
    next: Next,
) -> Result<Response, Json<serde_json::Value>> {
    // 模拟可能失败的请求
    if let Some(query) = original_uri.query() {
        if query.eq("number=11") {
            return Err(Json(json!({ "status": "ok" })));
        }
    }

    let method = request.method().clone();
    // let uri = request.uri().clone();
    let headers = request.headers().clone();
    // 记录请求开始时间
    let start = Instant::now();
    // 打印请求信息
    info!(
        "[Request] {} {} - Headers: {:?}",
        method, original_uri, headers
    );

    // 处理请求
    let response = next.run(request).await;
    // 记录响应信息
    let duration = start.elapsed();

    info!(
        "[Response] {} {} - Status: {} - Duration: {:?}",
        method,
        original_uri,
        response.status(),
        duration
    );

    Ok(response)
}

/// 限流，每秒超过100个就延迟
pub async fn rate_limiter(request: Request, next: Next) -> Result<Response, StatusCode> {
    // 简单的计数器限流
    static REQUEST_COUNT: AtomicU32 = AtomicU32::new(0);
    const MAX_REQUESTS: u32 = 100;

    let current = REQUEST_COUNT.fetch_add(1, Ordering::SeqCst);

    if current >= MAX_REQUESTS {
        // 模拟延迟或返回429
        sleep(Duration::from_millis(100)).await;
        // 或者直接返回错误
        // return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    let response = next.run(request).await;

    // 每秒清零（实际应用中需要更复杂的逻辑）
    Ok(response)
}
