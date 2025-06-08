use axum::{
    extract::{FromRequest, Request},
    Json,
};
use serde::de::DeserializeOwned;
use validator::{Validate, ValidationErrors};

use super::{error::ValidationError as ApiValidationError, types::ApiError};

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

        value.validate().map_err(validation_errors_to_api_error)?;

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

/// Validated path parameter extractor for security hardening
#[derive(Debug)]
pub struct ValidatedPath<T>(pub T);

impl<T> std::ops::Deref for ValidatedPath<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for ValidatedPath<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Secure validated run ID path parameter
#[derive(Debug)]
pub struct ValidatedRunId(pub String);

impl std::ops::Deref for ValidatedRunId {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> FromRequest<S> for ValidatedRunId
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Path(run_id) = axum::extract::Path::<String>::from_request(req, state)
            .await
            .map_err(|_| ApiError::bad_request("Missing run ID".to_string()))?;

        ValidationHelper::validate_run_id(&run_id)
            .map_err(|e| ApiError::bad_request(format!("Invalid run ID: {}", e)))?;

        Ok(ValidatedRunId(run_id))
    }
}

/// Secure validated codebase name path parameter
#[derive(Debug)]
pub struct ValidatedCodebase(pub String);

impl std::ops::Deref for ValidatedCodebase {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> FromRequest<S> for ValidatedCodebase
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Path(codebase) = axum::extract::Path::<String>::from_request(req, state)
            .await
            .map_err(|_| ApiError::bad_request("Missing codebase name".to_string()))?;

        ValidationHelper::validate_codebase_name(&codebase)
            .map_err(|e| ApiError::bad_request(format!("Invalid codebase name: {}", e)))?;

        Ok(ValidatedCodebase(codebase))
    }
}

/// Secure validated campaign name path parameter
#[derive(Debug)]
pub struct ValidatedCampaign(pub String);

impl std::ops::Deref for ValidatedCampaign {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> FromRequest<S> for ValidatedCampaign
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Path(campaign) = axum::extract::Path::<String>::from_request(req, state)
            .await
            .map_err(|_| ApiError::bad_request("Missing campaign name".to_string()))?;

        ValidationHelper::validate_campaign_name(&campaign)
            .map_err(|e| ApiError::bad_request(format!("Invalid campaign name: {}", e)))?;

        Ok(ValidatedCampaign(campaign))
    }
}

/// Secure validated filename path parameter
#[derive(Debug)]
pub struct ValidatedFilename(pub String);

impl std::ops::Deref for ValidatedFilename {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> FromRequest<S> for ValidatedFilename
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Path(filename) = axum::extract::Path::<String>::from_request(req, state)
            .await
            .map_err(|_| ApiError::bad_request("Missing filename".to_string()))?;

        ValidationHelper::validate_filename(&filename)
            .map_err(|e| ApiError::bad_request(format!("Invalid filename: {}", e)))?;

        Ok(ValidatedFilename(filename))
    }
}

/// Secure validated user ID path parameter
#[derive(Debug)]
pub struct ValidatedUserId(pub String);

impl std::ops::Deref for ValidatedUserId {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> FromRequest<S> for ValidatedUserId
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Path(user_id) = axum::extract::Path::<String>::from_request(req, state)
            .await
            .map_err(|_| ApiError::bad_request("Missing user ID".to_string()))?;

        ValidationHelper::validate_user_id(&user_id)
            .map_err(|e| ApiError::bad_request(format!("Invalid user ID: {}", e)))?;

        Ok(ValidatedUserId(user_id))
    }
}

/// Secure validated session ID path parameter
#[derive(Debug)]
pub struct ValidatedSessionId(pub String);

impl std::ops::Deref for ValidatedSessionId {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> FromRequest<S> for ValidatedSessionId
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Path(session_id) = axum::extract::Path::<String>::from_request(req, state)
            .await
            .map_err(|_| ApiError::bad_request("Missing session ID".to_string()))?;

        ValidationHelper::validate_session_id(&session_id)
            .map_err(|e| ApiError::bad_request(format!("Invalid session ID: {}", e)))?;

        Ok(ValidatedSessionId(session_id))
    }
}

/// Secure validated worker ID path parameter
#[derive(Debug)]
pub struct ValidatedWorkerId(pub String);

impl std::ops::Deref for ValidatedWorkerId {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> FromRequest<S> for ValidatedWorkerId
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Path(worker_id) = axum::extract::Path::<String>::from_request(req, state)
            .await
            .map_err(|_| ApiError::bad_request("Missing worker ID".to_string()))?;

        ValidationHelper::validate_worker_id(&worker_id)
            .map_err(|e| ApiError::bad_request(format!("Invalid worker ID: {}", e)))?;

        Ok(ValidatedWorkerId(worker_id))
    }
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
        if !(-10000..=10000).contains(&offset) {
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
        if !(1..=1000).contains(&limit) {
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

    /// Validate filename format (security-critical to prevent path traversal)
    pub fn validate_filename(filename: &str) -> Result<(), ApiValidationError> {
        if filename.is_empty() {
            return Err(ApiValidationError::MissingField {
                field: "filename".to_string(),
            });
        }

        if filename.len() > 255 {
            return Err(ApiValidationError::InvalidField {
                field: "filename".to_string(),
                reason: "Filename too long (max 255 characters)".to_string(),
            });
        }

        // Security check: prevent path traversal attacks
        if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
            return Err(ApiValidationError::InvalidField {
                field: "filename".to_string(),
                reason: "Filename contains unsafe path characters".to_string(),
            });
        }

        // Check for control characters and other unsafe characters
        if filename.chars().any(|c| c.is_control() || c == '\0') {
            return Err(ApiValidationError::InvalidField {
                field: "filename".to_string(),
                reason: "Filename contains control characters".to_string(),
            });
        }

        // Basic filename validation (alphanumeric, dash, underscore, dot)
        if !filename
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(ApiValidationError::InvalidField {
                field: "filename".to_string(),
                reason: "Filename contains invalid characters".to_string(),
            });
        }

        Ok(())
    }

    /// Validate user ID format (UUID or alphanumeric string)
    pub fn validate_user_id(user_id: &str) -> Result<(), ApiValidationError> {
        if user_id.is_empty() {
            return Err(ApiValidationError::MissingField {
                field: "user_id".to_string(),
            });
        }

        if user_id.len() < 3 || user_id.len() > 64 {
            return Err(ApiValidationError::InvalidField {
                field: "user_id".to_string(),
                reason: "Invalid user ID format".to_string(),
            });
        }

        // Allow alphanumeric, dash, underscore
        if !user_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ApiValidationError::InvalidField {
                field: "user_id".to_string(),
                reason: "User ID contains invalid characters".to_string(),
            });
        }

        Ok(())
    }

    /// Validate session ID format
    pub fn validate_session_id(session_id: &str) -> Result<(), ApiValidationError> {
        if session_id.is_empty() {
            return Err(ApiValidationError::MissingField {
                field: "session_id".to_string(),
            });
        }

        // Session IDs should be long enough to be secure
        if session_id.len() < 16 || session_id.len() > 128 {
            return Err(ApiValidationError::InvalidField {
                field: "session_id".to_string(),
                reason: "Invalid session ID format".to_string(),
            });
        }

        // Allow alphanumeric, dash, underscore
        if !session_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(ApiValidationError::InvalidField {
                field: "session_id".to_string(),
                reason: "Session ID contains invalid characters".to_string(),
            });
        }

        Ok(())
    }

    /// Validate worker ID format
    pub fn validate_worker_id(worker_id: &str) -> Result<(), ApiValidationError> {
        if worker_id.is_empty() {
            return Err(ApiValidationError::MissingField {
                field: "worker_id".to_string(),
            });
        }

        if worker_id.len() < 3 || worker_id.len() > 64 {
            return Err(ApiValidationError::InvalidField {
                field: "worker_id".to_string(),
                reason: "Invalid worker ID format".to_string(),
            });
        }

        // Allow alphanumeric, dash, underscore, dot (for hostnames)
        if !worker_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(ApiValidationError::InvalidField {
                field: "worker_id".to_string(),
                reason: "Worker ID contains invalid characters".to_string(),
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

    /// Validate filename
    pub fn validate_filename(filename: &str) -> Result<(), ValidationError> {
        super::ValidationHelper::validate_filename(filename)
            .map_err(|_| ValidationError::new("invalid_filename"))
    }

    /// Validate user ID
    pub fn validate_user_id(user_id: &str) -> Result<(), ValidationError> {
        super::ValidationHelper::validate_user_id(user_id)
            .map_err(|_| ValidationError::new("invalid_user_id"))
    }

    /// Validate session ID
    pub fn validate_session_id(session_id: &str) -> Result<(), ValidationError> {
        super::ValidationHelper::validate_session_id(session_id)
            .map_err(|_| ValidationError::new("invalid_session_id"))
    }

    /// Validate worker ID
    pub fn validate_worker_id(worker_id: &str) -> Result<(), ValidationError> {
        super::ValidationHelper::validate_worker_id(worker_id)
            .map_err(|_| ValidationError::new("invalid_worker_id"))
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
        assert!(!ValidationHelper::validate_bool_string("0").unwrap());
        assert!(ValidationHelper::validate_bool_string("1").unwrap());
        assert!(ValidationHelper::validate_bool_string("true").unwrap());
        assert!(!ValidationHelper::validate_bool_string("false").unwrap());
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

    #[test]
    fn test_filename_validation() {
        // Valid filenames
        assert!(ValidationHelper::validate_filename("file.txt").is_ok());
        assert!(ValidationHelper::validate_filename("test-file_v2.log").is_ok());
        assert!(ValidationHelper::validate_filename("data123.json").is_ok());

        // Invalid filenames - path traversal attempts
        assert!(ValidationHelper::validate_filename("../config").is_err());
        assert!(ValidationHelper::validate_filename("..\\windows\\file").is_err());
        assert!(ValidationHelper::validate_filename("subdir/file.txt").is_err());
        assert!(ValidationHelper::validate_filename("file\\test.txt").is_err());

        // Invalid filenames - control characters
        assert!(ValidationHelper::validate_filename("file\x00.txt").is_err());
        assert!(ValidationHelper::validate_filename("file\n.txt").is_err());

        // Invalid filenames - special characters
        assert!(ValidationHelper::validate_filename("file@test.txt").is_err());
        assert!(ValidationHelper::validate_filename("file space.txt").is_err());
        
        // Empty filename
        assert!(ValidationHelper::validate_filename("").is_err());
    }

    #[test]
    fn test_user_id_validation() {
        // Valid user IDs
        assert!(ValidationHelper::validate_user_id("user123").is_ok());
        assert!(ValidationHelper::validate_user_id("test-user_1").is_ok());
        assert!(ValidationHelper::validate_user_id("admin").is_ok());

        // Invalid user IDs
        assert!(ValidationHelper::validate_user_id("").is_err()); // Empty
        assert!(ValidationHelper::validate_user_id("ab").is_err()); // Too short
        assert!(ValidationHelper::validate_user_id("a".repeat(70).as_str()).is_err()); // Too long
        assert!(ValidationHelper::validate_user_id("user@domain.com").is_err()); // Invalid chars
        assert!(ValidationHelper::validate_user_id("user space").is_err()); // Space
    }

    #[test]
    fn test_session_id_validation() {
        // Valid session IDs
        assert!(ValidationHelper::validate_session_id("abc123def456ghi789jkl012").is_ok());
        assert!(ValidationHelper::validate_session_id("session-id_12345678901234567890").is_ok());

        // Invalid session IDs
        assert!(ValidationHelper::validate_session_id("").is_err()); // Empty
        assert!(ValidationHelper::validate_session_id("short").is_err()); // Too short
        assert!(ValidationHelper::validate_session_id("a".repeat(150).as_str()).is_err()); // Too long
        assert!(ValidationHelper::validate_session_id("session@id").is_err()); // Invalid chars
    }

    #[test]
    fn test_worker_id_validation() {
        // Valid worker IDs
        assert!(ValidationHelper::validate_worker_id("worker1").is_ok());
        assert!(ValidationHelper::validate_worker_id("host-1.example.com").is_ok());
        assert!(ValidationHelper::validate_worker_id("build_worker_123").is_ok());

        // Invalid worker IDs
        assert!(ValidationHelper::validate_worker_id("").is_err()); // Empty
        assert!(ValidationHelper::validate_worker_id("ab").is_err()); // Too short
        assert!(ValidationHelper::validate_worker_id("a".repeat(70).as_str()).is_err()); // Too long
        assert!(ValidationHelper::validate_worker_id("worker@host").is_err()); // Invalid chars
    }
}
