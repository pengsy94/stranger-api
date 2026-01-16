use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub token_type: String,
    pub message: String,
}