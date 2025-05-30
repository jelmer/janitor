use axum::{
    body::Body,
    extract::{FromRequest, Request},
    http,
    Json,
};
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use validator::{Validate, ValidationErrors};

use super::{
    error::ValidationError as ApiValidationError,
    types::{ApiError, ApiResponse},
};

/// Validated JSON extractor that automatically validates incoming JSON data
#[derive(Debug)]
pub struct ValidatedJson<T>(pub T);

impl<T> std::ops::Deref for ValidatedJson<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for ValidatedJson<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T, S> FromRequest<S> for ValidatedJson<T>
where
    T: DeserializeOwned + Validate + Send,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|_| ApiError::bad_request("Invalid JSON format".to_string()))?;

        value
            .validate()
            .map_err(|e| validation_errors_to_api_error(e))?;

        Ok(ValidatedJson(value))
    }
}

/// Convert validator ValidationErrors to ApiError
fn validation_errors_to_api_error(errors: ValidationErrors) -> ApiError {
    let mut error_details = serde_json::Map::new();
    
    for (field, field_errors) in errors.field_errors() {
        let field_error_messages: Vec<String> = field_errors
            .iter()
            .map(|error| {
                error
                    .message
                    .as_ref()
                    .map(|msg| msg.to_string())
                    .unwrap_or_else(|| format!("Validation failed for field: {}", field))
            })
            .collect();
        
        error_details.insert(
            field.to_string(),
            serde_json::Value::Array(
                field_error_messages
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );
    }
    
    ApiError::bad_request("Validation failed".to_string())
        .with_details(serde_json::Value::Object(error_details))
}

/// Validation helper for common patterns
pub struct ValidationHelper;

impl ValidationHelper {
    /// Validate boolean string (accepts "0", "1", "true", "false")
    pub fn validate_bool_string(value: &str) -> Result<bool, ApiValidationError> {
        match value.to_lowercase().as_str() {
            "0" | "false" => Ok(false),
            "1" | "true" => Ok(true),
            _ => Err(ApiValidationError::InvalidField {
                field: "boolean".to_string(),
                reason: "Must be '0', '1', 'true', or 'false'".to_string(),
            }),
        }
    }

    /// Validate codebase name format
    pub fn validate_codebase_name(name: &str) -> Result<(), ApiValidationError> {
        if name.is_empty() {
            return Err(ApiValidationError::MissingField {
                field: "codebase".to_string(),
            });
        }

        if name.len() > 255 {
            return Err(ApiValidationError::InvalidField {
                field: "codebase".to_string(),
                reason: "Name too long (max 255 characters)".to_string(),
            });
        }

        // Check for valid characters (alphanumeric, dash, underscore, dot)
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(ApiValidationError::InvalidField {
                field: "codebase".to_string(),
                reason: "Name contains invalid characters".to_string(),
            });
        }

        Ok(())
    }

    /// Validate campaign name format
    pub fn validate_campaign_name(name: &str) -> Result<(), ApiValidationError> {
        if name.is_empty() {
            return Err(ApiValidationError::MissingField {
                field: "campaign".to_string(),
            });
        }

        if name.len() > 255 {
            return Err(ApiValidationError::InvalidField {
                field: "campaign".to_string(),
                reason: "Name too long (max 255 characters)".to_string(),
            });
        }

        // Check for valid characters (alphanumeric, dash, underscore)
        if !name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ApiValidationError::InvalidField {
                field: "campaign".to_string(),
                reason: "Name contains invalid characters".to_string(),
            });
        }

        Ok(())
    }

    /// Validate run ID format (UUID-like string)
    pub fn validate_run_id(run_id: &str) -> Result<(), ApiValidationError> {
        if run_id.is_empty() {
            return Err(ApiValidationError::MissingField {
                field: "run_id".to_string(),
            });
        }

        // Simple validation - should be a reasonable length and contain valid characters
        if run_id.len() < 8 || run_id.len() > 64 {
            return Err(ApiValidationError::InvalidField {
                field: "run_id".to_string(),
                reason: "Invalid run ID format".to_string(),
            });
        }

        // Allow alphanumeric, dash, underscore
        if !run_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ApiValidationError::InvalidField {
                field: "run_id".to_string(),
                reason: "Run ID contains invalid characters".to_string(),
            });
        }

        Ok(())
    }

    /// Validate offset parameter (for pagination and queue positioning)
    pub fn validate_offset(offset: i32) -> Result<(), ApiValidationError> {
        // Allow reasonable range for queue positioning
        if offset < -10000 || offset > 10000 {
            return Err(ApiValidationError::OutOfRange {
                field: "offset".to_string(),
                min: -10000,
                max: 10000,
            });
        }

        Ok(())
    }

    /// Validate limit parameter (for pagination)
    pub fn validate_limit(limit: i64) -> Result<(), ApiValidationError> {
        if limit < 1 || limit > 1000 {
            return Err(ApiValidationError::OutOfRange {
                field: "limit".to_string(),
                min: 1,
                max: 1000,
            });
        }

        Ok(())
    }

    /// Validate duration in seconds
    pub fn validate_duration_seconds(duration: i32) -> Result<(), ApiValidationError> {
        if duration < 0 {
            return Err(ApiValidationError::InvalidField {
                field: "duration".to_string(),
                reason: "Duration cannot be negative".to_string(),
            });
        }

        // Reasonable upper limit (24 hours)
        if duration > 86400 {
            return Err(ApiValidationError::InvalidField {
                field: "duration".to_string(),
                reason: "Duration too long (max 24 hours)".to_string(),
            });
        }

        Ok(())
    }

    /// Validate email format (basic validation)
    pub fn validate_email(email: &str) -> Result<(), ApiValidationError> {
        if email.is_empty() {
            return Err(ApiValidationError::MissingField {
                field: "email".to_string(),
            });
        }

        if !email.contains('@') || !email.contains('.') {
            return Err(ApiValidationError::InvalidField {
                field: "email".to_string(),
                reason: "Invalid email format".to_string(),
            });
        }

        if email.len() > 255 {
            return Err(ApiValidationError::InvalidField {
                field: "email".to_string(),
                reason: "Email too long".to_string(),
            });
        }

        Ok(())
    }
}

/// Custom validator functions for use with the validator crate
pub mod validators {
    use validator::ValidationError;

    /// Validate codebase name
    pub fn validate_codebase_name(name: &str) -> Result<(), ValidationError> {
        super::ValidationHelper::validate_codebase_name(name)
            .map_err(|_| ValidationError::new("invalid_codebase_name"))
    }

    /// Validate campaign name
    pub fn validate_campaign_name(name: &str) -> Result<(), ValidationError> {
        super::ValidationHelper::validate_campaign_name(name)
            .map_err(|_| ValidationError::new("invalid_campaign_name"))
    }

    /// Validate run ID
    pub fn validate_run_id(run_id: &str) -> Result<(), ValidationError> {
        super::ValidationHelper::validate_run_id(run_id)
            .map_err(|_| ValidationError::new("invalid_run_id"))
    }

    /// Validate offset
    pub fn validate_offset(offset: i32) -> Result<(), ValidationError> {
        super::ValidationHelper::validate_offset(offset)
            .map_err(|_| ValidationError::new("invalid_offset"))
    }

    /// Validate duration
    pub fn validate_duration_seconds(duration: i32) -> Result<(), ValidationError> {
        super::ValidationHelper::validate_duration_seconds(duration)
            .map_err(|_| ValidationError::new("invalid_duration"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use validator::Validate;

    #[derive(Debug, Serialize, Deserialize, Validate)]
    struct TestStruct {
        #[validate(length(min = 1, max = 10))]
        name: String,
        #[validate(range(min = 0, max = 100))]
        value: i32,
    }

    #[test]
    fn test_validation_helper_bool_string() {
        assert_eq!(ValidationHelper::validate_bool_string("0").unwrap(), false);
        assert_eq!(ValidationHelper::validate_bool_string("1").unwrap(), true);
        assert_eq!(ValidationHelper::validate_bool_string("true").unwrap(), true);
        assert_eq!(ValidationHelper::validate_bool_string("false").unwrap(), false);
        assert!(ValidationHelper::validate_bool_string("invalid").is_err());
    }

    #[test]
    fn test_validation_helper_codebase_name() {
        assert!(ValidationHelper::validate_codebase_name("valid-name").is_ok());
        assert!(ValidationHelper::validate_codebase_name("valid_name").is_ok());
        assert!(ValidationHelper::validate_codebase_name("valid.name").is_ok());
        assert!(ValidationHelper::validate_codebase_name("").is_err());
        assert!(ValidationHelper::validate_codebase_name("invalid name").is_err());
        assert!(ValidationHelper::validate_codebase_name("invalid@name").is_err());
    }

    #[test]
    fn test_validation_helper_limits() {
        assert!(ValidationHelper::validate_offset(100).is_ok());
        assert!(ValidationHelper::validate_offset(-100).is_ok());
        assert!(ValidationHelper::validate_offset(20000).is_err());
        assert!(ValidationHelper::validate_offset(-20000).is_err());

        assert!(ValidationHelper::validate_limit(50).is_ok());
        assert!(ValidationHelper::validate_limit(1).is_ok());
        assert!(ValidationHelper::validate_limit(1000).is_ok());
        assert!(ValidationHelper::validate_limit(0).is_err());
        assert!(ValidationHelper::validate_limit(2000).is_err());
    }

    #[test]
    fn test_validation_errors_conversion() {
        let test_struct = TestStruct {
            name: "".to_string(), // Too short
            value: 150,           // Too high
        };

        let validation_result = test_struct.validate();
        assert!(validation_result.is_err());

        let errors = validation_result.unwrap_err();
        let api_error = validation_errors_to_api_error(errors);
        
        assert_eq!(api_error.error, "bad_request");
        assert!(api_error.details.is_some());
    }
}