pub mod query;
pub mod path;
pub mod json;
pub mod form;

use crate::utils::response::FieldError;
use validator::{ValidationErrors, ValidationErrorsKind};

/// validator -> FieldError 转换
pub fn validation_errors_to_fields(err: ValidationErrors) -> Vec<FieldError> {
    let mut errors = Vec::new();

    for (field, kind) in err.errors() {
        if let ValidationErrorsKind::Field(field_errors) = kind {
            for fe in field_errors {
                let msg = fe.message.clone().unwrap_or_else(|| "参数校验失败".into());

                errors.push(FieldError {
                    field: field.to_string(),
                    message: msg.to_string(),
                })
            }
        }
    }

    errors
}
