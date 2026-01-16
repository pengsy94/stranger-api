use crate::websocket::models::ConnectionManager;
use axum::Router;
use axum::routing::get;
use std::sync::Arc;

pub mod models;
pub mod ws;

/// websocket app 路由
pub fn set_websocket_api(connection_manager: Arc<ConnectionManager>) -> Router {
    Router::new()
        .route("/", get(ws::websocket_handler))
        .with_state(connection_manager)
}
