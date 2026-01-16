use crate::utils::response::{ErrorResponse, FieldError};
use crate::validator::validation_errors_to_fields;

use axum::{
    Json,
    extract::{FromRequest, Request},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;
use validator::Validate;

pub struct ValidatedJson<T>(pub T);

impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    S: Send + Sync,
    T: Validate + DeserializeOwned,
{
    type Rejection = Response;

    fn from_request(
        req: Request,
        state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        Box::pin(async move {
            let Json(value) = Json::<T>::from_request(req, state).await.map_err(|e| {
                return (
                    StatusCode::OK,
                    Json(ErrorResponse {
                        code: 500,
                        message: "Json 参数解析失败".into(),
                        errors: Some(vec![FieldError {
                            field: "Json".into(),
                            message: e.to_string(),
                        }]),
                    }),
                )
                    .into_response();
            })?;

            if let Err(err) = value.validate() {
                return Err((
                    StatusCode::OK,
                    Json(ErrorResponse {
                        code: 500,
                        message: "Json 参数校验失败".into(),
                        errors: Some(validation_errors_to_fields(err)),
                    }),
                )
                    .into_response());
            };

            Ok(ValidatedJson(value))
        })
    }
}
