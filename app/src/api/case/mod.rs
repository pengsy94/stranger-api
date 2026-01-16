use axum::{middleware, Router};
use axum::routing::{get, post};
use middleware_fn::request::logging_middleware;
use crate::api;

pub mod args;

pub fn set_test_api() -> Router {
    Router::new()
        // 获取参数 /{id}
        .route("/{id}", get(api::case::args::sys_path_test))
        .route("/{name}/{age}", get(api::case::args::sys_path_2_test))
        .route("/query", get(api::case::args::sys_query_test))
        // header获取
        .route("/header", get(api::case::args::sys_header_test))
        // 返回json
        .route("/json", get(api::case::args::sys_response_json))
        // post json提交参数
        .route("/post-json", post(api::case::args::sys_query_json))
        // post form提交参数
        .route("/post-form", post(api::case::args::sys_query_form))
        // 整个组添加 中间件案例
        .layer(middleware::from_fn(logging_middleware))
}
