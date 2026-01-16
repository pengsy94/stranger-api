use axum::http::header::USER_AGENT;
use axum::{
    Form,
    extract::{Path, Query},
    http::HeaderMap,
    response::{Html, Json},
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Deserialize, Debug, Serialize)]
pub struct Page {
    number: i32,
}

pub async fn sys_test() -> Html<&'static str> {
    tracing::info!("/app/test");
    Html("<html><h1>Hello, test!</h1><h2>你好, 测试!</h2></html>")
}

pub async fn sys_path_test(Path(_id): Path<u32>) -> &'static str {
    tracing::info!("sys_path_test: {}", _id);

    "sys_path_test"
}

pub async fn sys_path_2_test(Path((_name, _age)): Path<(String, u32)>) -> &'static str {
    tracing::info!("sys_path_2_test: name = {}, age = {}", _name, _age);

    "sys_path_2_test"
}

pub async fn sys_query_test(Query(_page): Query<Page>) -> &'static str {
    tracing::info!("sys_query_test: {:?}", _page);

    "sys_query_test"
}

pub async fn sys_header_test(header: HeaderMap) -> Json<Value> {
    tracing::info!("sys_header_test: {:?}", header);
    let user_agent = header
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.to_string())
        .unwrap();

    Json(json!({
        "user_agent" : user_agent,
    }))
}

pub async fn sys_response_json() -> Json<Page> {
    Json(Page { number: 88896565 })
}

pub async fn sys_query_json(Json(query): Json<Page>) -> Json<Page> {
    Json(Page {
        number: query.number,
    })
}

pub async fn sys_query_form(Form(query): Form<Page>) -> Json<Page> {
    Json(Page {
        number: query.number,
    })
}
