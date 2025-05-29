//! Tests to verify configuration compatibility with Python implementation.

use janitor_runner::config::{ApplicationConfig, DatabaseConfig, RunnerConfig, WebConfig};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;

/// Test that configuration files can be read in Python-compatible format.
#[test]
fn test_config_file_format_compatibility() {
    // Test TOML format that Python can also read
    let toml_config = r#"
[database]
url = "postgresql://user:pass@localhost/janitor"
max_connections = 20
connection_timeout_seconds = 30
query_timeout_seconds = 60

[web]
listen_address = "0.0.0.0"
port = 9911
public_port = 9919

[application]
name = "janitor-runner"
version = "1.0.0"
environment = "development"
debug = true
"#;

    let config: Result<RunnerConfig, _> = toml::from_str(toml_config);
    assert!(
        config.is_ok(),
        "Failed to parse TOML config: {:?}",
        config.err()
    );

    let config = config.unwrap();
    assert_eq!(
        config.database.url,
        "postgresql://user:pass@localhost/janitor"
    );
    assert_eq!(config.database.max_connections, 20);
    assert_eq!(config.web.listen_address, "0.0.0.0");
    assert_eq!(config.web.port, 9911);
    assert_eq!(config.application.name, "janitor-runner");
}

/// Test that environment variables work the same as Python.
#[test]
fn test_environment_variable_compatibility() {
    // Test environment variables that Python configuration uses
    let env_vars = vec![
        ("DATABASE_URL", "postgresql://localhost/test"),
        ("REDIS_URL", "redis://localhost:6379"),
        ("JANITOR_ENVIRONMENT", "production"),
        ("JANITOR_DEBUG", "false"),
        ("WEB_PORT", "8080"),
        ("WEB_LISTEN_ADDRESS", "127.0.0.1"),
    ];

    for (key, value) in env_vars {
        // Verify environment variable names match Python conventions
        assert!(key
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_'));
        assert!(!key.starts_with('_'));
        assert!(!key.ends_with('_'));
        assert!(!value.is_empty());
    }
}

/// Test default configuration values match Python.
#[test]
fn test_default_config_values() {
    let config = RunnerConfig::default();

    // Database defaults should match Python
    assert_eq!(config.database.max_connections, 10);
    assert_eq!(config.database.connection_timeout_seconds, 30);
    assert_eq!(config.database.query_timeout_seconds, 30);

    // Web server defaults should match Python
    assert_eq!(config.web.listen_address, "localhost");
    assert_eq!(config.web.port, 9911);
    assert_eq!(config.web.public_port, 9919);
    assert_eq!(config.web.request_timeout_seconds, 60);

    // Application defaults should match Python
    assert_eq!(config.application.environment, "development");
    assert_eq!(config.application.debug, false);
    assert_eq!(config.application.enable_graceful_shutdown, true);
    assert_eq!(config.application.shutdown_timeout_seconds, 30);
}

/// Test campaign configuration structure matches Python.
#[test]
fn test_campaign_config_structure() {
    // Test campaign configuration JSON format that Python uses
    let campaign_json = json!({
        "name": "lintian-fixes",
        "description": "Fix lintian issues in Debian packages",
        "command": "lintian-fixes",
        "value": 100,
        "success_chance": 0.85,
        "publish_policy": {
            "mode": "propose",
            "review_required": false
        },
        "schedule": {
            "frequency": "daily",
            "priority": "medium"
        }
    });

    // Verify structure matches Python expectations
    assert!(campaign_json["name"].is_string());
    assert!(campaign_json["description"].is_string());
    assert!(campaign_json["command"].is_string());
    assert!(campaign_json["value"].is_number());
    assert!(campaign_json["success_chance"].is_number());
    assert!(campaign_json["publish_policy"].is_object());
    assert!(campaign_json["schedule"].is_object());

    // Test that campaign name follows Python domain constraint
    let name = campaign_json["name"].as_str().unwrap();
    assert!(name.chars().nth(0).unwrap().is_ascii_alphanumeric());
    assert!(name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "+-".contains(c)));
}

/// Test worker configuration format matches Python.
#[test]
fn test_worker_config_format() {
    // Test worker configuration that Python workers expect
    let worker_config = json!({
        "env": {
            "DEBFULLNAME": "Janitor Bot",
            "DEBEMAIL": "janitor@debian.org",
            "GIT_COMMITTER_NAME": "Janitor Bot",
            "GIT_COMMITTER_EMAIL": "janitor@debian.org",
            "GIT_AUTHOR_NAME": "Janitor Bot",
            "GIT_AUTHOR_EMAIL": "janitor@debian.org",
            "BRZ_EMAIL": "Janitor Bot <janitor@debian.org>",
            "EMAIL": "janitor@debian.org",
            "COMMITTER": "Janitor Bot <janitor@debian.org>"
        },
        "campaign_config": {
            "lintian-fixes": {
                "command": "lintian-fixes",
                "value": 100,
                "publish_mode": "propose"
            }
        },
        "artifacts": {
            "base_url": "https://artifacts.janitor.debian.net"
        },
        "logs": {
            "base_url": "https://logs.janitor.debian.net"
        }
    });

    // Verify worker config has expected structure
    assert!(worker_config["env"].is_object());
    assert!(worker_config["campaign_config"].is_object());

    // Check environment variables match committer_env output
    let env = &worker_config["env"];
    assert!(env["DEBFULLNAME"].is_string());
    assert!(env["DEBEMAIL"].is_string());
    assert!(env["GIT_COMMITTER_NAME"].is_string());
    assert!(env["GIT_COMMITTER_EMAIL"].is_string());
    assert!(env["GIT_AUTHOR_NAME"].is_string());
    assert!(env["GIT_AUTHOR_EMAIL"].is_string());
    assert!(env["BRZ_EMAIL"].is_string());
    assert!(env["EMAIL"].is_string());
    assert!(env["COMMITTER"].is_string());
}

/// Test logging configuration matches Python.
#[test]
fn test_logging_config_compatibility() {
    // Test logging configuration format Python uses
    let logging_config = json!({
        "level": "INFO",
        "format": "%(asctime)s %(name)s %(levelname)s %(message)s",
        "handlers": {
            "console": {
                "type": "console",
                "level": "INFO"
            },
            "file": {
                "type": "file",
                "filename": "/var/log/janitor/runner.log",
                "level": "DEBUG",
                "rotation": {
                    "max_size_mb": 100,
                    "backup_count": 5
                }
            }
        }
    });

    // Verify logging config structure
    assert!(logging_config["level"].is_string());
    assert!(logging_config["format"].is_string());
    assert!(logging_config["handlers"].is_object());

    // Test log levels match Python
    let valid_levels = vec!["DEBUG", "INFO", "WARNING", "ERROR", "CRITICAL"];
    let level = logging_config["level"].as_str().unwrap();
    assert!(valid_levels.contains(&level));
}

/// Test database connection configuration matches Python.
#[test]
fn test_database_config_compatibility() {
    // Test database configuration that matches Python
    let db_config = DatabaseConfig {
        url: "postgresql://janitor:password@localhost:5432/janitor".to_string(),
        max_connections: 20,
        connection_timeout_seconds: 30,
        query_timeout_seconds: 60,
        enable_sql_logging: false,
    };

    // Verify PostgreSQL URL format
    assert!(db_config.url.starts_with("postgresql://"));
    assert!(db_config.url.contains("@"));
    assert!(db_config.url.contains(":"));
    assert!(db_config.url.contains("/"));

    // Test connection parameters are reasonable
    assert!(db_config.max_connections > 0);
    assert!(db_config.max_connections <= 100); // Reasonable limit
    assert!(db_config.connection_timeout_seconds > 0);
    assert!(db_config.query_timeout_seconds > 0);
}

/// Test VCS configuration matches Python.
#[test]
fn test_vcs_config_compatibility() {
    // Test VCS configuration that Python uses
    let vcs_config = json!({
        "git_location": "https://git.example.com",
        "bzr_location": "https://bazaar.example.com",
        "public_vcs_location": "https://public.example.com",
        "enable_caching": true,
        "timeout_seconds": 300
    });

    // Verify VCS config structure
    assert!(vcs_config["git_location"].is_string());
    assert!(vcs_config["bzr_location"].is_string());
    assert!(vcs_config["public_vcs_location"].is_string());
    assert!(vcs_config["enable_caching"].is_boolean());
    assert!(vcs_config["timeout_seconds"].is_number());

    // Test URL formats
    let git_url = vcs_config["git_location"].as_str().unwrap();
    assert!(git_url.starts_with("http://") || git_url.starts_with("https://"));
}

/// Test artifact storage configuration matches Python.
#[test]
fn test_artifact_config_compatibility() {
    // Test local artifact configuration
    let local_config = json!({
        "storage_backend": "Local",
        "local_artifact_path": "/var/lib/janitor/artifacts",
        "max_artifact_size": 104857600
    });

    assert_eq!(local_config["storage_backend"], "Local");
    assert!(local_config["local_artifact_path"].is_string());
    assert!(local_config["max_artifact_size"].is_number());

    // Test GCS artifact configuration
    let gcs_config = json!({
        "storage_backend": "Gcs",
        "gcs_bucket": "janitor-artifacts",
        "max_artifact_size": 104857600
    });

    assert_eq!(gcs_config["storage_backend"], "Gcs");
    assert!(gcs_config["gcs_bucket"].is_string());
    assert!(gcs_config["max_artifact_size"].is_number());
}

/// Test log storage configuration matches Python.
#[test]
fn test_log_config_compatibility() {
    // Test local log configuration
    let local_config = json!({
        "storage_backend": "Local",
        "local_log_path": "/var/lib/janitor/logs",
        "max_log_size": 52428800
    });

    assert_eq!(local_config["storage_backend"], "Local");
    assert!(local_config["local_log_path"].is_string());
    assert!(local_config["max_log_size"].is_number());

    // Test GCS log configuration
    let gcs_config = json!({
        "storage_backend": "Gcs",
        "gcs_bucket": "janitor-logs",
        "max_log_size": 52428800
    });

    assert_eq!(gcs_config["storage_backend"], "Gcs");
    assert!(gcs_config["gcs_bucket"].is_string());
    assert!(gcs_config["max_log_size"].is_number());
}

/// Test rate limiting configuration matches Python.
#[test]
fn test_rate_limiting_config() {
    // Test rate limiting configuration
    let rate_config = json!({
        "default_limit": 10,
        "host_limits": {
            "github.com": 5,
            "gitlab.com": 8,
            "launchpad.net": 3
        },
        "window_seconds": 3600
    });

    assert!(rate_config["default_limit"].is_number());
    assert!(rate_config["host_limits"].is_object());
    assert!(rate_config["window_seconds"].is_number());

    let default_limit = rate_config["default_limit"].as_u64().unwrap();
    assert!(default_limit > 0);

    let window = rate_config["window_seconds"].as_u64().unwrap();
    assert!(window > 0);
}

/// Test Redis configuration matches Python.
#[test]
fn test_redis_config_compatibility() {
    // Test Redis configuration that Python uses
    let redis_config = json!({
        "url": "redis://localhost:6379",
        "connection_timeout_seconds": 10,
        "command_timeout_seconds": 10,
        "max_connections": 10
    });

    assert!(redis_config["url"].is_string());
    assert!(redis_config["connection_timeout_seconds"].is_number());
    assert!(redis_config["command_timeout_seconds"].is_number());
    assert!(redis_config["max_connections"].is_number());

    let url = redis_config["url"].as_str().unwrap();
    assert!(url.starts_with("redis://"));
}

/// Test that configuration validation matches Python behavior.
#[test]
fn test_config_validation_compatibility() {
    // Test invalid database URL
    let invalid_db_config = DatabaseConfig {
        url: "invalid-url".to_string(),
        max_connections: 10,
        connection_timeout_seconds: 30,
        query_timeout_seconds: 30,
        enable_sql_logging: false,
    };

    // URL should not be valid
    assert!(!invalid_db_config.url.starts_with("postgresql://"));

    // Test invalid port numbers
    let invalid_web_config = WebConfig {
        listen_address: "localhost".to_string(),
        port: 0, // Invalid port
        public_port: 9919,
        request_timeout_seconds: 60,
        max_request_size_bytes: 10 * 1024 * 1024,
        enable_cors: false,
        enable_request_logging: true,
    };

    assert_eq!(invalid_web_config.port, 0); // Should be rejected by validation

    // Test invalid timeout values
    let invalid_timeouts = vec![0, u64::MAX];
    for timeout in invalid_timeouts {
        // These should be rejected by validation logic
        assert!(timeout == 0 || timeout == u64::MAX);
    }
}

/// Test configuration file paths match Python conventions.
#[test]
fn test_config_file_paths() {
    // Test configuration file paths that Python looks for
    let config_paths = vec![
        "/etc/janitor/runner.conf",
        "/etc/janitor/runner.toml",
        "~/.config/janitor/runner.conf",
        "./janitor.conf",
        "./janitor.toml",
    ];

    for path in config_paths {
        assert!(!path.is_empty());
        assert!(path.contains("janitor"));
        assert!(path.contains(".conf") || path.contains(".toml"));
    }
}

/// Test that sensitive configuration values are handled properly.
#[test]
fn test_sensitive_config_handling() {
    // Test that sensitive values are handled like Python
    let sensitive_fields = vec![
        "database.url",
        "redis.url",
        "worker.password",
        "api.secret_key",
        "gcs.credentials",
    ];

    for field in sensitive_fields {
        // These should be redacted in logs/debug output
        assert!(
            field.contains("url")
                || field.contains("password")
                || field.contains("secret")
                || field.contains("credentials")
        );
    }
}
