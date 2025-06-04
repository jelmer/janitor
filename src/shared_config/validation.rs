//! Configuration validation and error types

use std::fmt;

/// Configuration error types
#[derive(Debug, Clone)]
pub enum ConfigError {
    /// Required configuration value is missing
    MissingRequired(String),

    /// Failed to parse a configuration value
    ParseError { field: String, message: String },

    /// File I/O error
    IoError { path: String, message: String },

    /// Serialization/deserialization error
    SerdeError { format: String, message: String },

    /// Validation error
    ValidationError(ValidationError),

    /// Multiple errors occurred
    Multiple(Vec<ConfigError>),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingRequired(field) => {
                write!(f, "Required configuration field '{}' is missing", field)
            }
            ConfigError::ParseError { field, message } => {
                write!(f, "Failed to parse field '{}': {}", field, message)
            }
            ConfigError::IoError { path, message } => {
                write!(f, "I/O error reading config file '{}': {}", path, message)
            }
            ConfigError::SerdeError { format, message } => {
                write!(f, "Failed to parse {} configuration: {}", format, message)
            }
            ConfigError::ValidationError(err) => {
                write!(f, "Configuration validation error: {}", err)
            }
            ConfigError::Multiple(errors) => {
                write!(f, "Multiple configuration errors:\n")?;
                for (i, error) in errors.iter().enumerate() {
                    write!(f, "  {}: {}\n", i + 1, error)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<ValidationError> for ConfigError {
    fn from(err: ValidationError) -> Self {
        ConfigError::ValidationError(err)
    }
}

/// Configuration validation error
#[derive(Debug, Clone)]
pub enum ValidationError {
    /// A field has an invalid value
    InvalidValue { field: String, message: String },

    /// A required field is missing
    MissingField { field: String },

    /// Fields have conflicting values
    ConflictingFields {
        fields: Vec<String>,
        message: String,
    },

    /// A constraint is violated
    ConstraintViolation { constraint: String, message: String },

    /// Multiple validation errors
    Multiple(Vec<ValidationError>),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::InvalidValue { field, message } => {
                write!(f, "Invalid value for field '{}': {}", field, message)
            }
            ValidationError::MissingField { field } => {
                write!(f, "Required field '{}' is missing", field)
            }
            ValidationError::ConflictingFields { fields, message } => {
                write!(f, "Conflicting fields [{}]: {}", fields.join(", "), message)
            }
            ValidationError::ConstraintViolation {
                constraint,
                message,
            } => {
                write!(f, "Constraint '{}' violated: {}", constraint, message)
            }
            ValidationError::Multiple(errors) => {
                write!(f, "Multiple validation errors:\n")?;
                for (i, error) in errors.iter().enumerate() {
                    write!(f, "  {}: {}\n", i + 1, error)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validator utility for common validation patterns
pub struct Validator {
    errors: Vec<ValidationError>,
}

impl Validator {
    /// Create a new validator
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }

    /// Validate that a string field is not empty
    pub fn require_non_empty(&mut self, field: &str, value: &str) -> &mut Self {
        if value.is_empty() {
            self.errors.push(ValidationError::InvalidValue {
                field: field.to_string(),
                message: "Cannot be empty".to_string(),
            });
        }
        self
    }

    /// Validate that a numeric field is within a range
    pub fn require_range<T>(&mut self, field: &str, value: T, min: T, max: T) -> &mut Self
    where
        T: PartialOrd + fmt::Display + Copy,
    {
        if value < min || value > max {
            self.errors.push(ValidationError::InvalidValue {
                field: field.to_string(),
                message: format!("Must be between {} and {}, got {}", min, max, value),
            });
        }
        self
    }

    /// Validate that a numeric field is positive
    pub fn require_positive<T>(&mut self, field: &str, value: T) -> &mut Self
    where
        T: PartialOrd + Default + fmt::Display + Copy,
    {
        if value <= T::default() {
            self.errors.push(ValidationError::InvalidValue {
                field: field.to_string(),
                message: format!("Must be positive, got {}", value),
            });
        }
        self
    }

    /// Validate that a URL is valid
    pub fn require_valid_url(&mut self, field: &str, value: &str) -> &mut Self {
        if !value.is_empty() {
            if let Err(e) = value.parse::<url::Url>() {
                self.errors.push(ValidationError::InvalidValue {
                    field: field.to_string(),
                    message: format!("Invalid URL: {}", e),
                });
            }
        }
        self
    }

    /// Validate that a file path exists
    pub fn require_file_exists(&mut self, field: &str, path: &std::path::Path) -> &mut Self {
        if !path.exists() {
            self.errors.push(ValidationError::InvalidValue {
                field: field.to_string(),
                message: format!("File does not exist: {}", path.display()),
            });
        }
        self
    }

    /// Validate that a directory path exists
    pub fn require_dir_exists(&mut self, field: &str, path: &std::path::Path) -> &mut Self {
        if !path.exists() {
            self.errors.push(ValidationError::InvalidValue {
                field: field.to_string(),
                message: format!("Directory does not exist: {}", path.display()),
            });
        } else if !path.is_dir() {
            self.errors.push(ValidationError::InvalidValue {
                field: field.to_string(),
                message: format!("Path is not a directory: {}", path.display()),
            });
        }
        self
    }

    /// Add a custom validation error
    pub fn add_error(&mut self, error: ValidationError) -> &mut Self {
        self.errors.push(error);
        self
    }

    /// Add a custom validation error with field and message
    pub fn add_invalid_value(&mut self, field: &str, message: &str) -> &mut Self {
        self.errors.push(ValidationError::InvalidValue {
            field: field.to_string(),
            message: message.to_string(),
        });
        self
    }

    /// Finalize validation and return result
    pub fn finish(self) -> Result<(), ValidationError> {
        if self.errors.is_empty() {
            Ok(())
        } else if self.errors.len() == 1 {
            Err(self.errors.into_iter().next().unwrap())
        } else {
            Err(ValidationError::Multiple(self.errors))
        }
    }
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro for creating validation errors more concisely
#[macro_export]
macro_rules! validation_error {
    ($field:expr, $message:expr) => {
        ValidationError::InvalidValue {
            field: $field.to_string(),
            message: $message.to_string(),
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_require_non_empty() {
        let mut validator = Validator::new();

        validator.require_non_empty("field1", "valid");
        validator.require_non_empty("field2", "");

        let result = validator.finish();
        assert!(result.is_err());
    }

    #[test]
    fn test_validator_require_positive() {
        let mut validator = Validator::new();

        validator.require_positive("positive", 10);
        validator.require_positive("zero", 0);
        validator.require_positive("negative", -5);

        let result = validator.finish();
        assert!(result.is_err());
    }

    #[test]
    fn test_validator_multiple_errors() {
        let mut validator = Validator::new();

        validator.require_non_empty("field1", "");
        validator.require_positive("field2", 0);
        validator.require_range("field3", 100, 1, 50);

        let result = validator.finish();
        assert!(result.is_err());

        if let Err(ValidationError::Multiple(errors)) = result {
            assert_eq!(errors.len(), 3);
        } else {
            panic!("Expected multiple validation errors");
        }
    }

    #[test]
    fn test_validator_success() {
        let mut validator = Validator::new();

        validator.require_non_empty("field1", "valid");
        validator.require_positive("field2", 10);
        validator.require_range("field3", 25, 1, 50);

        let result = validator.finish();
        assert!(result.is_ok());
    }
}
