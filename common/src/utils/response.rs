use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::IntoResponse;
use axum::response::Response;
use serde::Serialize;
use std::fmt::Debug;

#[derive(Serialize, Clone, Debug)]
pub struct FieldError {
    pub field: String,
    pub message: String,
}

#[derive(Debug, Serialize, Default)]
pub struct ErrorResponse {
    pub code: i32,
    pub message: String,
    pub errors: Option<Vec<FieldError>>,
}

#[derive(Debug, Serialize, Default)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<FieldError>>,
}

/// 填入到extensions中的数据
#[derive(Debug, Clone)]
pub struct ResJsonString(pub String);

#[allow(unconditional_recursion)]
impl<T> IntoResponse for ApiResponse<T>
where
    T: Serialize + Send + Sync + Debug + 'static,
{
    fn into_response(self) -> Response {
        let data = Self {
            code: self.code,
            data: self.data,
            message: self.message,
            errors: self.errors,
        };
        let json_string = match serde_json::to_string(&data) {
            Ok(v) => v,
            Err(e) => {
                return Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(
                        header::CONTENT_TYPE,
                        HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
                    )
                    .body(Body::from(e.to_string()))
                    .unwrap();
            }
        };
        let res_json_string = ResJsonString(json_string.clone());
        // 构建响应
        let mut response = Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_static(mime::APPLICATION_JSON.as_ref()),
            )
            .body(Body::from(json_string))
            .unwrap();
        response.extensions_mut().insert(res_json_string);
        response
    }
}

impl<T: Serialize> ApiResponse<T> {
    /// 成功响应
    pub fn success(data: T) -> Self {
        Self {
            code: 200,
            message: "success".to_string(),
            data: Some(data),
            errors: None,
        }
    }

    /// 成功响应（带自定义消息）
    pub fn success_with_message(data: T, message: &str) -> Self {
        Self {
            code: 200,
            message: message.to_string(),
            data: Some(data),
            errors: None,
        }
    }

    /// 从 ErrorResponse 创建错误响应
    pub fn from_error_response(error: ErrorResponse) -> Self {
        Self {
            code: error.code,
            message: error.message,
            data: None,
            errors: error.errors,
        }
    }

    /// 创建错误响应
    pub fn error(code: i32, message: &str) -> Self {
        Self {
            code,
            message: message.to_string(),
            data: None,
            errors: None,
        }
    }

    /// 创建带字段错误的错误响应
    pub fn error_with_errors(code: i32, message: &str, errors: Vec<FieldError>) -> Self {
        Self {
            code,
            message: message.to_string(),
            data: None,
            errors: Some(errors),
        }
    }

    /// 创建带单个字段错误的错误响应
    pub fn error_with_field_error(
        code: i32,
        message: &str,
        field: &str,
        field_message: &str,
    ) -> Self {
        let field_error = FieldError {
            field: field.to_string(),
            message: field_message.to_string(),
        };

        Self {
            code,
            message: message.to_string(),
            data: None,
            errors: Some(vec![field_error]),
        }
    }

    /// 检查是否是成功响应
    pub fn is_success(&self) -> bool {
        self.code == 0
    }

    /// 获取数据（如果存在）
    pub fn get_data(&self) -> Option<&T> {
        self.data.as_ref()
    }

    /// 获取字段错误
    pub fn get_errors(&self) -> Option<&Vec<FieldError>> {
        self.errors.as_ref()
    }
}

// 为特殊类型实现便捷方法
impl<T: Serialize> ApiResponse<T> {
    /// 转换为 ErrorResponse（当只有错误信息时）
    pub fn to_error_response(&self) -> Option<ErrorResponse> {
        if self.data.is_none() {
            Some(ErrorResponse {
                code: self.code,
                message: self.message.clone(),
                errors: self.errors.clone(),
            })
        } else {
            None
        }
    }
}

// 为 () 类型实现特殊方法（无数据返回的情况）
impl ApiResponse<()> {
    /// 创建成功响应（无数据）
    pub fn ok() -> Self {
        Self {
            code: 200,
            message: "success".to_string(),
            data: None,
            errors: None,
        }
    }

    /// 创建成功响应（带消息，无数据）
    pub fn ok_with_message(message: &str) -> Self {
        Self {
            code: 200,
            message: message.to_string(),
            data: None,
            errors: None,
        }
    }
}
