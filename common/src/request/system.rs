use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Deserialize, Serialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "无效的邮箱地址"))]
    pub email: String,

    #[validate(length(min = 6, message = "密码必须至少包含6个字符"))]
    pub password: String,

    #[validate()]
    pub remember_me: Option<bool>,
}