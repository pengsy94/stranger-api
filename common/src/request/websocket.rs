use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize)]
pub struct WsRequestParams {
    pub key: String
}