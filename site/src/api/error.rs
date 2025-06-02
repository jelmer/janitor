use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{error, warn};

use super::types::{ApiError, ApiResponse};

/// API error types for consistent error categorization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiErrorType {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    UnprocessableEntity,
    InternalError,
    ServiceUnavailable,
    GatewayTimeout,
    BadGateway,
}

impl ApiErrorType {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest => StatusCode::BAD_REQUEST,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict => StatusCode::CONFLICT,
            Self::UnprocessableEntity => StatusCode::UNPROCESSABLE_ENTITY,
            Self::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::GatewayTimeout => StatusCode::GATEWAY_TIMEOUT,
            Self::BadGateway => StatusCode::BAD_GATEWAY,
        }
    }
}

/// Service communication errors
#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Connection error: {0}")]
    Connection(#[from] reqwest::Error),

    #[error("Timeout error: {service} service timeout")]
    Timeout { service: String },

    #[error("Service returned error: {status} - {message}")]
    ServiceError { status: u16, message: String },

    #[error("Invalid response format: {0}")]
    InvalidResponse(String),

    #[error("Service unavailable: {service}")]
    Unavailable { service: String },
}

impl ServiceError {
    pub fn to_api_error(&self) -> ApiError {
        match self {
            Self::Connection(e) => {
                error!("Service connection error: {}", e);
                ApiError::service_unavailable("external service".to_string())
            }
            Self::Timeout { service } => {
                warn!("Service timeout: {}", service);
                ApiError::gateway_timeout(service.clone())
            }
            Self::ServiceError { status, message } => {
                warn!("Service error: {} - {}", status, message);
                match *status {
                    400..=499 => ApiError::bad_request(message.clone()),
                    500..=599 => {
                        ApiError::new("service_error".to_string(), StatusCode::BAD_GATEWAY)
                            .with_reason(message.clone())
                    }
                    _ => ApiError::internal_error(message.clone()),
                }
            }
            Self::InvalidResponse(msg) => {
                error!("Invalid service response: {}", msg);
                ApiError::new("invalid_response".to_string(), StatusCode::BAD_GATEWAY)
                    .with_reason(msg.clone())
            }
            Self::Unavailable { service } => {
                warn!("Service unavailable: {}", service);
                ApiError::service_unavailable(service.clone())
            }
        }
    }
}

/// Database operation errors
#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Database query error: {0}")]
    Query(#[from] sqlx::Error),

    #[error("Record not found: {resource}")]
    NotFound { resource: String },

    #[error("Constraint violation: {0}")]
    Constraint(String),

    #[error("Transaction error: {0}")]
    Transaction(String),
}

impl DatabaseError {
    pub fn to_api_error(&self) -> ApiError {
        match self {
            Self::Query(e) => {
                error!("Database query error: {}", e);

                // Check for specific PostgreSQL error codes
                if let Some(db_err) = e.as_database_error() {
                    match db_err.code().as_ref().map(|s| s.as_ref()) {
                        Some("23505") => {
                            // unique_violation
                            ApiError::new("conflict".to_string(), StatusCode::CONFLICT)
                                .with_reason("Resource already exists".to_string())
                        }
                        Some("23503") => {
                            // foreign_key_violation
                            ApiError::bad_request("Referenced resource does not exist".to_string())
                        }
                        _ => ApiError::internal_error("Database operation failed".to_string()),
                    }
                } else {
                    ApiError::internal_error("Database operation failed".to_string())
                }
            }
            Self::NotFound { resource } => ApiError::not_found(resource.clone()),
            Self::Constraint(msg) => ApiError::bad_request(msg.clone()),
            Self::Transaction(msg) => {
                error!("Database transaction error: {}", msg);
                ApiError::internal_error("Transaction failed".to_string())
            }
        }
    }
}

/// Validation errors
#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Missing required field: {field}")]
    MissingField { field: String },

    #[error("Invalid field value: {field} - {reason}")]
    InvalidField { field: String, reason: String },

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Value out of range: {field} must be between {min} and {max}")]
    OutOfRange { field: String, min: i64, max: i64 },
}

impl ValidationError {
    pub fn to_api_error(&self) -> ApiError {
        match self {
            Self::MissingField { field } => {
                ApiError::bad_request(format!("Missing required field: {}", field))
            }
            Self::InvalidField { field, reason } => {
                ApiError::bad_request(format!("Invalid {}: {}", field, reason))
            }
            Self::InvalidFormat(msg) => ApiError::bad_request(format!("Invalid format: {}", msg)),
            Self::OutOfRange { field, min, max } => {
                ApiError::bad_request(format!("{} must be between {} and {}", field, min, max))
            }
        }
    }
}

/// Combined error type for API operations
#[derive(Debug, Error)]
pub enum AppError {
    #[error("Service error: {0}")]
    Service(#[from] ServiceError),

    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    #[error("Validation error: {0}")]
    Validation(#[from] ValidationError),

    #[error("Authentication error: {0}")]
    Auth(#[from] crate::auth::AuthError),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl AppError {
    pub fn to_api_error(&self) -> ApiError {
        match self {
            Self::Service(e) => e.to_api_error(),
            Self::Database(e) => e.to_api_error(),
            Self::Validation(e) => e.to_api_error(),
            Self::Auth(e) => {
                warn!("Authentication error: {}", e);
                match e {
                    crate::auth::AuthError::SessionNotFound => ApiError::unauthorized(),
                    crate::auth::AuthError::InsufficientPermissions => ApiError::forbidden(),
                    _ => ApiError::unauthorized(),
                }
            }
            Self::Config(msg) => {
                error!("Configuration error: {}", msg);
                ApiError::internal_error("Configuration error".to_string())
            }
            Self::Io(e) => {
                error!("IO error: {}", e);
                ApiError::internal_error("File operation failed".to_string())
            }
            Self::Json(e) => {
                warn!("JSON parsing error: {}", e);
                ApiError::bad_request("Invalid JSON format".to_string())
            }
        }
    }
}

/// Convert AppError to API response
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let api_error = self.to_api_error();
        let status =
            StatusCode::from_u16(api_error.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let response = ApiResponse::<()>::error_with_details(
            api_error.error.clone(),
            api_error.reason.clone(),
            api_error.details.unwrap_or_else(|| serde_json::json!({})),
        );

        (status, Json(response)).into_response()
    }
}

/// Convert ApiError to API response
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        let response = ApiResponse::<()>::error_with_details(
            self.error.clone(),
            self.reason.clone(),
            self.details.unwrap_or_else(|| serde_json::json!({})),
        );

        (status, Json(response)).into_response()
    }
}

/// Helper function to handle service communication errors
pub fn handle_service_error(service_name: &str, error: reqwest::Error) -> ApiError {
    if error.is_timeout() {
        ApiError::gateway_timeout(service_name.to_string())
    } else if error.is_connect() {
        ApiError::service_unavailable(service_name.to_string())
    } else {
        error!(
            "Service communication error with {}: {}",
            service_name, error
        );
        ApiError::new("service_error".to_string(), StatusCode::BAD_GATEWAY)
            .with_reason(format!("Communication error with {}", service_name))
    }
}

/// Helper for creating validation errors
pub fn validation_error(field: &str, reason: &str) -> ValidationError {
    ValidationError::InvalidField {
        field: field.to_string(),
        reason: reason.to_string(),
    }
}

/// Helper for creating not found errors
pub fn not_found_error(resource: &str) -> DatabaseError {
    DatabaseError::NotFound {
        resource: resource.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_error_type_status_codes() {
        assert_eq!(
            ApiErrorType::BadRequest.status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(ApiErrorType::NotFound.status_code(), StatusCode::NOT_FOUND);
        assert_eq!(
            ApiErrorType::InternalError.status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn test_validation_error_conversion() {
        let validation_err = ValidationError::MissingField {
            field: "name".to_string(),
        };

        let api_err = validation_err.to_api_error();
        assert_eq!(api_err.error, "bad_request");
        assert!(api_err
            .reason
            .unwrap()
            .contains("Missing required field: name"));
    }

    #[test]
    fn test_service_error_conversion() {
        let service_err = ServiceError::Unavailable {
            service: "runner".to_string(),
        };

        let api_err = service_err.to_api_error();
        assert_eq!(api_err.error, "service_unavailable");
        assert_eq!(api_err.status, 503);
    }
}
