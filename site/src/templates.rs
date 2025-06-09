use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use tera::{Context, Tera, Value as TeraValue};
use url::Url;

use crate::{
    auth::types::{SessionInfo, User, UserRole},
    config::SiteConfig,
};

pub mod helpers;

#[cfg(test)]
mod test;

/// Flash message categories for user feedback
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FlashCategory {
    Success,
    Info,
    Warning,
    Error,
}

impl FlashCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            FlashCategory::Success => "success",
            FlashCategory::Info => "info", 
            FlashCategory::Warning => "warning",
            FlashCategory::Error => "error",
        }
    }
}

impl std::fmt::Display for FlashCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Flash message structure for user feedback
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FlashMessage {
    pub category: FlashCategory,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub dismissible: bool,
}

impl FlashMessage {
    pub fn new(category: FlashCategory, message: String) -> Self {
        Self {
            category,
            message,
            timestamp: Utc::now(),
            dismissible: true,
        }
    }

    pub fn success(message: String) -> Self {
        Self::new(FlashCategory::Success, message)
    }

    pub fn info(message: String) -> Self {
        Self::new(FlashCategory::Info, message)
    }

    pub fn warning(message: String) -> Self {
        Self::new(FlashCategory::Warning, message)
    }

    pub fn error(message: String) -> Self {
        Self::new(FlashCategory::Error, message)
    }

    pub fn non_dismissible(mut self) -> Self {
        self.dismissible = false;
        self
    }
}

/// Flash message storage key for session temporary data
pub const FLASH_MESSAGES_KEY: &str = "flash_messages";

pub fn setup_templates(config: &SiteConfig) -> Result<Tera> {
    let template_dir = config.template_directory();

    let template_pattern = format!("{}/**/*.html", template_dir);
    let mut tera = Tera::new(&template_pattern)?;

    // Configure Tera for Jinja2 compatibility
    tera.autoescape_on(vec!["html", "xml"]);

    // Register custom filters that match Python Jinja2 implementation
    tera.register_filter("basename", basename_filter);
    tera.register_filter("timeago", timeago_filter);
    tera.register_filter("duration", format_duration_filter);
    tera.register_filter("summarize", summarize_filter);
    tera.register_filter("safe", safe_filter);
    tera.register_filter("timestamp", format_timestamp_filter);
    tera.register_filter("tojson", tojson_filter);

    // Register global functions that match Python implementation
    tera.register_function("url_for", url_for_function);
    tera.register_function("get_flashed_messages", get_flashed_messages_function);
    tera.register_function("utcnow", utcnow_function);
    tera.register_function("enumerate", enumerate_function);
    tera.register_function("format_duration", format_duration_function);
    tera.register_function("format_timestamp", format_timestamp_function);
    tera.register_function("classify_result_code", classify_result_code_function);
    tera.register_function("worker_link_is_global", worker_link_is_global_function);

    Ok(tera)
}

// Custom filters to match Python Jinja2 implementation
fn basename_filter(value: &TeraValue, _: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    let path_str = value
        .as_str()
        .ok_or_else(|| tera::Error::msg("Value must be a string"))?;
    let basename = std::path::Path::new(path_str)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path_str);
    Ok(TeraValue::String(basename.to_string()))
}

fn timeago_filter(value: &TeraValue, _: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    // Parse datetime and format as time ago
    if let Some(dt_str) = value.as_str() {
        if let Ok(dt) = DateTime::parse_from_rfc3339(dt_str) {
            let now = Utc::now();
            let duration = now.signed_duration_since(dt.with_timezone(&Utc));
            return Ok(TeraValue::String(format_duration_string(duration)));
        }
    }
    Ok(value.clone())
}

fn format_duration_filter(
    value: &TeraValue,
    _: &HashMap<String, TeraValue>,
) -> tera::Result<TeraValue> {
    if let Some(seconds) = value.as_f64() {
        let duration = Duration::seconds(seconds as i64);
        Ok(TeraValue::String(format_duration_string(duration)))
    } else {
        Ok(value.clone())
    }
}

fn summarize_filter(
    value: &TeraValue,
    args: &HashMap<String, TeraValue>,
) -> tera::Result<TeraValue> {
    let text = value
        .as_str()
        .ok_or_else(|| tera::Error::msg("Value must be a string"))?;
    let length = args.get("length").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

    if text.len() <= length {
        Ok(value.clone())
    } else {
        let truncated = text.chars().take(length).collect::<String>();
        Ok(TeraValue::String(format!("{}...", truncated)))
    }
}

fn safe_filter(value: &TeraValue, _: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    // Mark content as safe (already escaped HTML)
    if let Some(text) = value.as_str() {
        Ok(TeraValue::String(text.to_string()))
    } else {
        Ok(value.clone())
    }
}

fn format_timestamp_filter(
    value: &TeraValue,
    _: &HashMap<String, TeraValue>,
) -> tera::Result<TeraValue> {
    if let Some(dt_str) = value.as_str() {
        if let Ok(dt) = DateTime::parse_from_rfc3339(dt_str) {
            return Ok(TeraValue::String(dt.format("%Y-%m-%d %H:%M").to_string()));
        }
    }
    Ok(value.clone())
}

fn tojson_filter(value: &TeraValue, args: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    let indent = args.get("indent").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

    let json_value: JsonValue = match value {
        TeraValue::String(s) => JsonValue::String(s.clone()),
        TeraValue::Number(n) => JsonValue::Number(n.clone()),
        TeraValue::Bool(b) => JsonValue::Bool(*b),
        TeraValue::Array(a) => JsonValue::Array(a.iter().map(tera_to_json).collect()),
        TeraValue::Object(o) => JsonValue::Object(
            o.iter()
                .map(|(k, v)| (k.clone(), tera_to_json(v)))
                .collect(),
        ),
        TeraValue::Null => JsonValue::Null,
    };

    let json_str = if indent > 0 {
        serde_json::to_string_pretty(&json_value)
    } else {
        serde_json::to_string(&json_value)
    }
    .map_err(|e| tera::Error::msg(format!("JSON serialization error: {}", e)))?;

    Ok(TeraValue::String(json_str))
}

// Helper function to convert Tera values to JSON
fn tera_to_json(value: &TeraValue) -> JsonValue {
    match value {
        TeraValue::String(s) => JsonValue::String(s.clone()),
        TeraValue::Number(n) => JsonValue::Number(n.clone()),
        TeraValue::Bool(b) => JsonValue::Bool(*b),
        TeraValue::Array(a) => JsonValue::Array(a.iter().map(tera_to_json).collect()),
        TeraValue::Object(o) => JsonValue::Object(
            o.iter()
                .map(|(k, v)| (k.clone(), tera_to_json(v)))
                .collect(),
        ),
        TeraValue::Null => JsonValue::Null,
    }
}

// Global functions to match Python implementation
fn url_for_function(args: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    let endpoint = args
        .get("endpoint")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("endpoint argument required"))?;

    // Generate URLs based on actual route structure
    let url = generate_url(endpoint, args)?;
    Ok(TeraValue::String(url))
}

/// Generate URLs based on endpoint name and parameters
fn generate_url(endpoint: &str, args: &HashMap<String, TeraValue>) -> tera::Result<String> {
    match endpoint {
        // Static pages
        "index" => Ok("/".to_string()),
        "about" => Ok("/about".to_string()),
        "credentials" => Ok("/credentials".to_string()),
        
        // API endpoints
        "api_health" => Ok("/api/v1/health".to_string()),
        "api_status" => Ok("/api/v1/status".to_string()),
        "api_runs" => Ok("/api/v1/runs".to_string()),
        "api_workers" => Ok("/api/v1/workers".to_string()),
        
        // Archive endpoints
        "archive_keyring_asc" => Ok("/archive-keyring.asc".to_string()),
        "archive_keyring_gpg" => Ok("/archive-keyring.gpg".to_string()),
        
        // VCS repository lists
        "git_repo_list" => Ok("/git/".to_string()),
        "bzr_repo_list" => Ok("/bzr/".to_string()),
        
        // Campaign routes
        "campaign_start" => {
            let campaign = get_param(args, "campaign")?;
            Ok(format!("/{}/", campaign))
        },
        "campaign_candidates" => {
            let suite = get_param(args, "suite")?;
            Ok(format!("/{}/candidates", suite))
        },
        "ready_list" => {
            let suite = get_param(args, "suite")?;
            Ok(format!("/{}/ready", suite))
        },
        "done_list" => {
            let campaign = get_param(args, "campaign")?;
            Ok(format!("/{}/done", campaign))
        },
        "merge_proposals" => {
            let suite = get_param(args, "suite")?;
            Ok(format!("/{}/merge-proposals", suite))
        },
        
        // Codebase routes
        "codebase_detail" => {
            let campaign = get_param(args, "campaign")?;
            let codebase = get_param(args, "codebase")?;
            Ok(format!("/{}/c/{}/", campaign, codebase))
        },
        "run_detail" => {
            let campaign = get_param(args, "campaign")?;
            let codebase = get_param(args, "codebase")?;
            let run_id = get_param(args, "run_id")?;
            Ok(format!("/{}/c/{}/{}", campaign, codebase, run_id))
        },
        
        // Log and diff routes
        "view_log" => {
            let campaign = get_param(args, "campaign")?;
            let codebase = get_param(args, "codebase")?;
            let run_id = get_param(args, "run_id")?;
            let log_name = get_param(args, "log_name")?;
            Ok(format!("/{}/c/{}/{}/logs/{}", campaign, codebase, run_id, log_name))
        },
        "download_log" => {
            let campaign = get_param(args, "campaign")?;
            let codebase = get_param(args, "codebase")?;
            let run_id = get_param(args, "run_id")?;
            let log_name = get_param(args, "log_name")?;
            Ok(format!("/{}/c/{}/{}/logs/{}/download", campaign, codebase, run_id, log_name))
        },
        "view_diff" => {
            let campaign = get_param(args, "campaign")?;
            let codebase = get_param(args, "codebase")?;
            let run_id = get_param(args, "run_id")?;
            Ok(format!("/{}/c/{}/{}/diff", campaign, codebase, run_id))
        },
        "view_debdiff" => {
            let campaign = get_param(args, "campaign")?;
            let codebase = get_param(args, "codebase")?;
            let run_id = get_param(args, "run_id")?;
            Ok(format!("/{}/c/{}/{}/debdiff", campaign, codebase, run_id))
        },
        
        // Cupboard admin routes
        "cupboard_dashboard" => Ok("/cupboard/".to_string()),
        "cupboard_review" => Ok("/cupboard/review/".to_string()),
        "cupboard_publish" => Ok("/cupboard/publish/".to_string()),
        "cupboard_queue" => Ok("/cupboard/queue/".to_string()),
        "cupboard_workers" => Ok("/cupboard/workers/".to_string()),
        
        // Legacy routes
        "pkg_list" => Ok("/pkg".to_string()),
        "pkg_detail" => {
            let name = get_param(args, "name")?;
            Ok(format!("/pkg/{}", name))
        },
        
        // Webhook routes
        "webhook_github" => Ok("/webhook/github".to_string()),
        "webhook_gitlab" => Ok("/webhook/gitlab".to_string()),
        
        // Default case for unknown endpoints
        _ => {
            tracing::warn!("Unknown endpoint for URL generation: {}", endpoint);
            Ok(format!("/{}", endpoint))
        }
    }
}

/// Helper function to extract required parameters from template arguments
fn get_param(args: &HashMap<String, TeraValue>, param: &str) -> tera::Result<String> {
    args.get(param)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| tera::Error::msg(format!("Required parameter '{}' not provided", param)))
}

fn get_flashed_messages_function(args: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    // Extract category filter if provided
    let category_filter = args.get("category_filter").and_then(|v| v.as_str());
    
    // For now, return empty array - this will be populated by middleware
    // that has access to the session and database
    let messages = if let Some(messages) = args.get("_flash_messages") {
        if let Some(messages_array) = messages.as_array() {
            if let Some(filter) = category_filter {
                // Filter messages by category
                let filtered: Vec<TeraValue> = messages_array
                    .iter()
                    .filter(|msg| {
                        msg.as_object()
                            .and_then(|obj| obj.get("category"))
                            .and_then(|cat| cat.as_str())
                            .map(|cat| cat == filter)
                            .unwrap_or(false)
                    })
                    .cloned()
                    .collect();
                TeraValue::Array(filtered)
            } else {
                messages.clone()
            }
        } else {
            TeraValue::Array(vec![])
        }
    } else {
        TeraValue::Array(vec![])
    };
    
    Ok(messages)
}

fn utcnow_function(_args: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    let now = Utc::now();
    Ok(TeraValue::String(now.to_rfc3339()))
}

fn enumerate_function(args: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    let array = args
        .get("array")
        .and_then(|v| v.as_array())
        .ok_or_else(|| tera::Error::msg("array argument required"))?;

    let start = args.get("start").and_then(|v| v.as_u64()).unwrap_or(0);

    let enumerated: Vec<TeraValue> = array
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let mut obj = tera::Map::new();
            obj.insert(
                "0".to_string(),
                TeraValue::Number((i as u64 + start).into()),
            );
            obj.insert("1".to_string(), item.clone());
            TeraValue::Object(obj)
        })
        .collect();

    Ok(TeraValue::Array(enumerated))
}

fn format_duration_function(args: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    let duration_seconds = args
        .get("duration")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| tera::Error::msg("duration argument required"))?;

    let duration = Duration::seconds(duration_seconds as i64);
    Ok(TeraValue::String(format_duration_string(duration)))
}

fn format_timestamp_function(args: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    let timestamp = args
        .get("timestamp")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("timestamp argument required"))?;

    if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp) {
        Ok(TeraValue::String(dt.format("%Y-%m-%d %H:%M").to_string()))
    } else {
        Ok(TeraValue::String(timestamp.to_string()))
    }
}

fn classify_result_code_function(args: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    let result_code = args
        .get("result_code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("result_code argument required"))?;

    let _transient = args
        .get("transient")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let classification = match result_code {
        "success" => "success",
        "failure" => "failure",
        "nothing-to-do" => "success",
        _ => "unknown",
    };

    Ok(TeraValue::String(classification.to_string()))
}

fn worker_link_is_global_function(args: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    let url_str = args
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("url argument required"))?;

    // Check if URL uses global IP address
    if let Ok(url) = Url::parse(url_str) {
        if let Some(host) = url.host_str() {
            // Simple check for private IP ranges
            let is_global = !host.starts_with("192.168.")
                && !host.starts_with("10.")
                && !host.starts_with("172.16.")
                && host != "localhost"
                && host != "127.0.0.1";
            return Ok(TeraValue::Bool(is_global));
        }
    }

    Ok(TeraValue::Bool(false))
}

// Helper function to format durations in human-readable format
fn format_duration_string(duration: Duration) -> String {
    let total_seconds = duration.num_seconds();

    if total_seconds < 60 {
        format!("{}s", total_seconds)
    } else if total_seconds < 3600 {
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        if seconds == 0 {
            format!("{}m", minutes)
        } else {
            format!("{}m{}s", minutes, seconds)
        }
    } else if total_seconds < 86400 {
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        if minutes == 0 {
            format!("{}h", hours)
        } else {
            format!("{}h{}m", hours, minutes)
        }
    } else {
        let days = total_seconds / 86400;
        let hours = (total_seconds % 86400) / 3600;
        if hours == 0 {
            format!("{}d", days)
        } else {
            format!("{}d{}h", days, hours)
        }
    }
}

pub fn create_base_context() -> Context {
    let mut context = Context::new();

    // Add global variables that are available in all templates
    context.insert("app_name", "Debian Janitor");
    context.insert("version", env!("CARGO_PKG_VERSION"));

    // Add common template functions as context variables
    let now = Utc::now();
    context.insert("utcnow", &now.to_rfc3339());

    context
}

pub fn create_base_context_with_config(config: &SiteConfig) -> Context {
    let mut context = create_base_context();
    
    // Add dynamic configuration flags
    let openid_configured = config.oidc_client_id.is_some() 
        && config.oidc_client_secret.is_some() 
        && (config.oidc_issuer_url.is_some() || config.oidc_base_url.is_some());
    context.insert("openid_configured", &openid_configured);
    
    context
}

pub fn create_request_context(base: Context, _request_path: &str) -> Context {
    create_request_context_with_flash(base, _request_path, None)
}

pub fn create_request_context_with_flash(
    base: Context, 
    _request_path: &str,
    flash_messages: Option<Vec<FlashMessage>>
) -> Context {
    create_request_context_with_session(base, _request_path, flash_messages, None, None, None)
}

pub fn create_request_context_with_session(
    base: Context,
    _request_path: &str,
    flash_messages: Option<Vec<FlashMessage>>,
    session_info: Option<&SessionInfo>,
    admin_group: Option<&str>,
    qa_reviewer_group: Option<&str>,
) -> Context {
    let mut context = base;

    // Add session-based user information
    if let Some(session) = session_info {
        let user = &session.user;
        
        // User role information
        context.insert("is_admin", &user.has_role(UserRole::Admin, admin_group, qa_reviewer_group));
        context.insert("is_qa_reviewer", &user.has_role(UserRole::QaReviewer, admin_group, qa_reviewer_group));
        
        // User information
        let user_display_name = user.name.as_ref()
            .or(user.preferred_username.as_ref())
            .unwrap_or(&user.email);
        context.insert("user", &Some(user_display_name));
        context.insert("user_email", &user.email);
        
        // Add user object for templates that need more details
        context.insert("user_info", &user);
    } else {
        // No session - anonymous user
        context.insert("is_admin", &false);
        context.insert("is_qa_reviewer", &false);
        context.insert("user", &Option::<String>::None);
        context.insert("user_email", &Option::<String>::None);
    }

    // Add campaign/suite configuration
    // TODO: Load from database - for now return empty arrays
    context.insert("suites", &Vec::<String>::new());
    context.insert("campaigns", &Vec::<String>::new());

    // Add flash messages if provided
    if let Some(messages) = flash_messages {
        let messages_json: Vec<TeraValue> = messages
            .into_iter()
            .map(|msg| {
                let mut map = tera::Map::new();
                map.insert("category".to_string(), TeraValue::String(msg.category.as_str().to_string()));
                map.insert("message".to_string(), TeraValue::String(msg.message));
                map.insert("timestamp".to_string(), TeraValue::String(msg.timestamp.to_rfc3339()));
                map.insert("dismissible".to_string(), TeraValue::Bool(msg.dismissible));
                TeraValue::Object(map)
            })
            .collect();
        context.insert("_flash_messages", &messages_json);
    } else {
        context.insert("_flash_messages", &Vec::<TeraValue>::new());
    }

    context
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tera::Value as TeraValue;

    #[test]
    fn test_generate_url_static_pages() {
        let args = HashMap::new();
        
        assert_eq!(generate_url("index", &args).unwrap(), "/");
        assert_eq!(generate_url("about", &args).unwrap(), "/about");
        assert_eq!(generate_url("credentials", &args).unwrap(), "/credentials");
    }

    #[test]
    fn test_generate_url_api_endpoints() {
        let args = HashMap::new();
        
        assert_eq!(generate_url("api_health", &args).unwrap(), "/api/v1/health");
        assert_eq!(generate_url("api_status", &args).unwrap(), "/api/v1/status");
        assert_eq!(generate_url("api_runs", &args).unwrap(), "/api/v1/runs");
    }

    #[test]
    fn test_generate_url_with_params() {
        let mut args = HashMap::new();
        args.insert("campaign".to_string(), TeraValue::String("lintian-fixes".to_string()));
        args.insert("codebase".to_string(), TeraValue::String("example-package".to_string()));
        args.insert("run_id".to_string(), TeraValue::String("run-123".to_string()));
        
        assert_eq!(
            generate_url("campaign_start", &args).unwrap(),
            "/lintian-fixes/"
        );
        
        assert_eq!(
            generate_url("codebase_detail", &args).unwrap(),
            "/lintian-fixes/c/example-package/"
        );
        
        assert_eq!(
            generate_url("run_detail", &args).unwrap(),
            "/lintian-fixes/c/example-package/run-123"
        );
    }

    #[test]
    fn test_generate_url_with_log_params() {
        let mut args = HashMap::new();
        args.insert("campaign".to_string(), TeraValue::String("lintian-fixes".to_string()));
        args.insert("codebase".to_string(), TeraValue::String("example-package".to_string()));
        args.insert("run_id".to_string(), TeraValue::String("run-123".to_string()));
        args.insert("log_name".to_string(), TeraValue::String("build.log".to_string()));
        
        assert_eq!(
            generate_url("view_log", &args).unwrap(),
            "/lintian-fixes/c/example-package/run-123/logs/build.log"
        );
        
        assert_eq!(
            generate_url("download_log", &args).unwrap(),
            "/lintian-fixes/c/example-package/run-123/logs/build.log/download"
        );
    }

    #[test]
    fn test_generate_url_cupboard_routes() {
        let args = HashMap::new();
        
        assert_eq!(generate_url("cupboard_dashboard", &args).unwrap(), "/cupboard/");
        assert_eq!(generate_url("cupboard_review", &args).unwrap(), "/cupboard/review/");
        assert_eq!(generate_url("cupboard_publish", &args).unwrap(), "/cupboard/publish/");
        assert_eq!(generate_url("cupboard_queue", &args).unwrap(), "/cupboard/queue/");
    }

    #[test]
    fn test_generate_url_missing_required_param() {
        let args = HashMap::new();
        
        // Should fail when required parameter is missing
        assert!(generate_url("campaign_start", &args).is_err());
        assert!(generate_url("codebase_detail", &args).is_err());
    }

    #[test]
    fn test_generate_url_unknown_endpoint() {
        let args = HashMap::new();
        
        // Should fallback to default behavior for unknown endpoints
        assert_eq!(generate_url("unknown_endpoint", &args).unwrap(), "/unknown_endpoint");
    }

    #[test]
    fn test_get_param_success() {
        let mut args = HashMap::new();
        args.insert("test_param".to_string(), TeraValue::String("test_value".to_string()));
        
        assert_eq!(get_param(&args, "test_param").unwrap(), "test_value");
    }

    #[test]
    fn test_get_param_missing() {
        let args = HashMap::new();
        
        assert!(get_param(&args, "missing_param").is_err());
    }

    #[test]
    fn test_url_for_function() {
        let mut args = HashMap::new();
        args.insert("endpoint".to_string(), TeraValue::String("index".to_string()));
        
        let result = url_for_function(&args).unwrap();
        assert_eq!(result, TeraValue::String("/".to_string()));
    }

    #[test]
    fn test_url_for_function_with_params() {
        let mut args = HashMap::new();
        args.insert("endpoint".to_string(), TeraValue::String("campaign_start".to_string()));
        args.insert("campaign".to_string(), TeraValue::String("test-campaign".to_string()));
        
        let result = url_for_function(&args).unwrap();
        assert_eq!(result, TeraValue::String("/test-campaign/".to_string()));
    }

    #[test]
    fn test_flash_message_creation() {
        let success_msg = FlashMessage::success("Operation completed successfully".to_string());
        assert_eq!(success_msg.category, FlashCategory::Success);
        assert_eq!(success_msg.message, "Operation completed successfully");
        assert!(success_msg.dismissible);

        let error_msg = FlashMessage::error("Something went wrong".to_string()).non_dismissible();
        assert_eq!(error_msg.category, FlashCategory::Error);
        assert_eq!(error_msg.message, "Something went wrong");
        assert!(!error_msg.dismissible);

        let info_msg = FlashMessage::info("Information message".to_string());
        assert_eq!(info_msg.category, FlashCategory::Info);

        let warning_msg = FlashMessage::warning("Warning message".to_string());
        assert_eq!(warning_msg.category, FlashCategory::Warning);
    }

    #[test]
    fn test_flash_category_display() {
        assert_eq!(FlashCategory::Success.as_str(), "success");
        assert_eq!(FlashCategory::Info.as_str(), "info");
        assert_eq!(FlashCategory::Warning.as_str(), "warning");
        assert_eq!(FlashCategory::Error.as_str(), "error");

        assert_eq!(format!("{}", FlashCategory::Success), "success");
        assert_eq!(format!("{}", FlashCategory::Error), "error");
    }

    #[test]
    fn test_get_flashed_messages_function_empty() {
        let args = HashMap::new();
        let result = get_flashed_messages_function(&args).unwrap();
        assert_eq!(result, TeraValue::Array(vec![]));
    }

    #[test]
    fn test_get_flashed_messages_function_with_messages() {
        let mut args = HashMap::new();
        
        // Create test flash messages
        let mut msg1 = tera::Map::new();
        msg1.insert("category".to_string(), TeraValue::String("success".to_string()));
        msg1.insert("message".to_string(), TeraValue::String("Success message".to_string()));
        msg1.insert("dismissible".to_string(), TeraValue::Bool(true));

        let mut msg2 = tera::Map::new();
        msg2.insert("category".to_string(), TeraValue::String("error".to_string()));
        msg2.insert("message".to_string(), TeraValue::String("Error message".to_string()));
        msg2.insert("dismissible".to_string(), TeraValue::Bool(false));

        let messages = vec![
            TeraValue::Object(msg1),
            TeraValue::Object(msg2),
        ];
        
        args.insert("_flash_messages".to_string(), TeraValue::Array(messages.clone()));
        
        let result = get_flashed_messages_function(&args).unwrap();
        assert_eq!(result, TeraValue::Array(messages));
    }

    #[test]
    fn test_get_flashed_messages_function_with_category_filter() {
        let mut args = HashMap::new();
        
        // Create test flash messages
        let mut success_msg = tera::Map::new();
        success_msg.insert("category".to_string(), TeraValue::String("success".to_string()));
        success_msg.insert("message".to_string(), TeraValue::String("Success message".to_string()));

        let mut error_msg = tera::Map::new();
        error_msg.insert("category".to_string(), TeraValue::String("error".to_string()));
        error_msg.insert("message".to_string(), TeraValue::String("Error message".to_string()));

        let messages = vec![
            TeraValue::Object(success_msg.clone()),
            TeraValue::Object(error_msg),
        ];
        
        args.insert("_flash_messages".to_string(), TeraValue::Array(messages));
        args.insert("category_filter".to_string(), TeraValue::String("success".to_string()));
        
        let result = get_flashed_messages_function(&args).unwrap();
        assert_eq!(result, TeraValue::Array(vec![TeraValue::Object(success_msg)]));
    }

    #[test]
    fn test_create_request_context_with_flash() {
        let base_context = create_base_context();
        let flash_messages = vec![
            FlashMessage::success("Test success".to_string()),
            FlashMessage::error("Test error".to_string()),
        ];

        let context = create_request_context_with_flash(base_context, "/test", Some(flash_messages));
        
        // Verify flash messages are included
        let messages = context.get("_flash_messages").unwrap();
        assert!(messages.is_array());
        
        let messages_array = messages.as_array().unwrap();
        assert_eq!(messages_array.len(), 2);
        
        // Check first message
        let first_msg = messages_array[0].as_object().unwrap();
        assert_eq!(first_msg.get("category").unwrap().as_str().unwrap(), "success");
        assert_eq!(first_msg.get("message").unwrap().as_str().unwrap(), "Test success");
        assert_eq!(first_msg.get("dismissible").unwrap().as_bool().unwrap(), true);
        
        // Check second message  
        let second_msg = messages_array[1].as_object().unwrap();
        assert_eq!(second_msg.get("category").unwrap().as_str().unwrap(), "error");
        assert_eq!(second_msg.get("message").unwrap().as_str().unwrap(), "Test error");
    }

    #[test]
    fn test_create_request_context_without_flash() {
        let base_context = create_base_context();
        let context = create_request_context_with_flash(base_context, "/test", None);
        
        // Verify empty flash messages array is included
        let messages = context.get("_flash_messages").unwrap();
        assert!(messages.is_array());
        assert_eq!(messages.as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_create_request_context_with_session() {
        use crate::auth::types::{SessionInfo, User};
        use std::collections::HashSet;

        let base_context = create_base_context();
        
        // Create test user with admin privileges
        let mut groups = HashSet::new();
        groups.insert("admins".to_string());
        
        let user = User {
            email: "admin@example.com".to_string(),
            name: Some("Admin User".to_string()),
            preferred_username: Some("admin".to_string()),
            groups,
            sub: "admin123".to_string(),
            additional_claims: serde_json::Map::new(),
        };
        
        let session_info = SessionInfo::new(user);
        
        let context = create_request_context_with_session(
            base_context,
            "/test",
            None,
            Some(&session_info),
            Some("admins"),
            Some("qa"),
        );
        
        // Verify user role information
        assert_eq!(context.get("is_admin").unwrap().as_bool().unwrap(), true);
        assert_eq!(context.get("is_qa_reviewer").unwrap().as_bool().unwrap(), true); // Admins are also QA reviewers
        assert_eq!(context.get("user").unwrap().as_str().unwrap(), "Admin User");
        assert_eq!(context.get("user_email").unwrap().as_str().unwrap(), "admin@example.com");
    }

    #[test]
    fn test_create_request_context_anonymous_user() {
        let base_context = create_base_context();
        
        let context = create_request_context_with_session(
            base_context,
            "/test",
            None,
            None, // No session
            Some("admins"),
            Some("qa"),
        );
        
        // Verify anonymous user has no privileges
        assert_eq!(context.get("is_admin").unwrap().as_bool().unwrap(), false);
        assert_eq!(context.get("is_qa_reviewer").unwrap().as_bool().unwrap(), false);
        assert!(context.get("user").unwrap().is_null());
    }

    #[test]
    fn test_create_base_context_with_config() {
        use crate::config::SiteConfig;
        
        // Test with OIDC configured
        let mut config = SiteConfig::default();
        config.oidc_client_id = Some("test_client".to_string());
        config.oidc_client_secret = Some("test_secret".to_string());
        config.oidc_issuer_url = Some("https://oidc.example.com".to_string());
        
        let context = create_base_context_with_config(&config);
        assert_eq!(context.get("openid_configured").unwrap().as_bool().unwrap(), true);
        
        // Test without OIDC configured
        let config_no_oidc = SiteConfig::default();
        let context_no_oidc = create_base_context_with_config(&config_no_oidc);
        assert_eq!(context_no_oidc.get("openid_configured").unwrap().as_bool().unwrap(), false);
    }
}
