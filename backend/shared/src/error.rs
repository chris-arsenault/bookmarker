use crate::config::ConfigError;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error(transparent)]
    Config(#[from] ConfigError),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("not found: {0}")]
    NotFound(String),

    #[error("database error: {0}")]
    Database(String),

    #[error("external service error from {service}: {message}")]
    ExternalService {
        service: &'static str,
        message: String,
    },

    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicError {
    pub status_code: u16,
    pub code: &'static str,
    pub message: String,
}

impl AppError {
    pub fn public_error(&self) -> PublicError {
        match self {
            Self::Config(_)
            | Self::Database(_)
            | Self::ExternalService { .. }
            | Self::Internal(_) => internal_error(),
            Self::Unauthorized(message) => PublicError {
                status_code: 401,
                code: "unauthorized",
                message: message.clone(),
            },
            Self::Validation(message) => PublicError {
                status_code: 400,
                code: "validation_error",
                message: message.clone(),
            },
            Self::NotFound(_) => PublicError {
                status_code: 404,
                code: "not_found",
                message: "not found".to_string(),
            },
        }
    }
}

fn internal_error() -> PublicError {
    PublicError {
        status_code: 500,
        code: "internal_error",
        message: "internal error".to_string(),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::config::ConfigError;

    use super::{AppError, PublicError};

    #[test]
    fn config_errors_map_to_public_internal_error() {
        let err = AppError::from(ConfigError::MissingEnv {
            name: "DB_PASSWORD",
        });

        assert_eq!(
            err.public_error(),
            PublicError {
                status_code: 500,
                code: "internal_error",
                message: "internal error".to_string(),
            }
        );
    }

    #[test]
    fn auth_errors_keep_safe_public_message() {
        let err = AppError::Unauthorized("missing bearer token".to_string());

        assert_eq!(
            err.public_error(),
            PublicError {
                status_code: 401,
                code: "unauthorized",
                message: "missing bearer token".to_string(),
            }
        );
    }

    #[test]
    fn not_found_errors_do_not_expose_internal_resource_details() {
        let err = AppError::NotFound("item 00000000-0000-0000-0000-000000000001".to_string());

        assert_eq!(
            err.public_error(),
            PublicError {
                status_code: 404,
                code: "not_found",
                message: "not found".to_string(),
            }
        );
    }

    #[test]
    fn database_errors_do_not_expose_internal_detail() {
        let err = AppError::Database("password was secret-value".to_string());

        assert_eq!(err.public_error().message, "internal error");
    }
}
