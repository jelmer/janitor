use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use tera::{Context, Tera, Value as TeraValue};
use url::Url;

use crate::config::Config;

pub fn setup_templates(config: &Config) -> Result<Tera> {
    let template_dir = config.template_dir
        .as_deref()
        .unwrap_or("py/janitor/site/templates");
    
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
    let path_str = value.as_str().ok_or_else(|| tera::Error::msg("Value must be a string"))?;
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

fn format_duration_filter(value: &TeraValue, _: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    if let Some(seconds) = value.as_f64() {
        let duration = Duration::seconds(seconds as i64);
        Ok(TeraValue::String(format_duration_string(duration)))
    } else {
        Ok(value.clone())
    }
}

fn summarize_filter(value: &TeraValue, args: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    let text = value.as_str().ok_or_else(|| tera::Error::msg("Value must be a string"))?;
    let length = args.get("length")
        .and_then(|v| v.as_u64())
        .unwrap_or(100) as usize;
    
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

fn format_timestamp_filter(value: &TeraValue, _: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    if let Some(dt_str) = value.as_str() {
        if let Ok(dt) = DateTime::parse_from_rfc3339(dt_str) {
            return Ok(TeraValue::String(dt.format("%Y-%m-%d %H:%M").to_string()));
        }
    }
    Ok(value.clone())
}

fn tojson_filter(value: &TeraValue, args: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    let indent = args.get("indent")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    
    let json_value: JsonValue = match value {
        TeraValue::String(s) => JsonValue::String(s.clone()),
        TeraValue::Number(n) => JsonValue::Number(n.clone()),
        TeraValue::Bool(b) => JsonValue::Bool(*b),
        TeraValue::Array(a) => JsonValue::Array(a.iter().map(tera_to_json).collect()),
        TeraValue::Object(o) => JsonValue::Object(
            o.iter().map(|(k, v)| (k.clone(), tera_to_json(v))).collect()
        ),
        TeraValue::Null => JsonValue::Null,
    };
    
    let json_str = if indent > 0 {
        serde_json::to_string_pretty(&json_value)
    } else {
        serde_json::to_string(&json_value)
    }.map_err(|e| tera::Error::msg(format!("JSON serialization error: {}", e)))?;
    
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
            o.iter().map(|(k, v)| (k.clone(), tera_to_json(v))).collect()
        ),
        TeraValue::Null => JsonValue::Null,
    }
}

// Global functions to match Python implementation
fn url_for_function(args: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    let endpoint = args.get("endpoint")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("endpoint argument required"))?;
    
    // TODO: Implement actual URL generation based on routes
    // For now, return a basic mapping
    let url = match endpoint {
        "index" => "/",
        "about" => "/about",
        "pkg_list" => "/pkg",
        "api_health" => "/api/v1/health",
        _ => &format!("/{}", endpoint),
    };
    
    Ok(TeraValue::String(url.to_string()))
}

fn get_flashed_messages_function(_args: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    // TODO: Implement flash message retrieval from session
    Ok(TeraValue::Array(vec![]))
}

fn utcnow_function(_args: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    let now = Utc::now();
    Ok(TeraValue::String(now.to_rfc3339()))
}

fn enumerate_function(args: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    let array = args.get("array")
        .and_then(|v| v.as_array())
        .ok_or_else(|| tera::Error::msg("array argument required"))?;
    
    let start = args.get("start")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    
    let enumerated: Vec<TeraValue> = array
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let mut obj = tera::Map::new();
            obj.insert("0".to_string(), TeraValue::Number((i as u64 + start).into()));
            obj.insert("1".to_string(), item.clone());
            TeraValue::Object(obj)
        })
        .collect();
    
    Ok(TeraValue::Array(enumerated))
}

fn format_duration_function(args: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    let duration_seconds = args.get("duration")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| tera::Error::msg("duration argument required"))?;
    
    let duration = Duration::seconds(duration_seconds as i64);
    Ok(TeraValue::String(format_duration_string(duration)))
}

fn format_timestamp_function(args: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    let timestamp = args.get("timestamp")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("timestamp argument required"))?;
    
    if let Ok(dt) = DateTime::parse_from_rfc3339(timestamp) {
        Ok(TeraValue::String(dt.format("%Y-%m-%d %H:%M").to_string()))
    } else {
        Ok(TeraValue::String(timestamp.to_string()))
    }
}

fn classify_result_code_function(args: &HashMap<String, TeraValue>) -> tera::Result<TeraValue> {
    let result_code = args.get("result_code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("result_code argument required"))?;
    
    let _transient = args.get("transient")
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
    let url_str = args.get("url")
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
    
    // Add configuration flags
    context.insert("openid_configured", &false); // TODO: Make this dynamic
    
    context
}

pub fn create_request_context(base: Context, _request_path: &str) -> Context {
    let mut context = base;
    
    // Add request-specific variables
    context.insert("is_admin", &false); // TODO: Get from session
    context.insert("is_qa_reviewer", &false); // TODO: Get from session
    context.insert("user", &Option::<String>::None); // TODO: Get from session
    
    // Add campaign/suite configuration
    // TODO: Load from database
    context.insert("suites", &Vec::<String>::new());
    context.insert("campaigns", &Vec::<String>::new());
    
    context
}