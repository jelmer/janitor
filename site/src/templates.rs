use anyhow::Result;
use tera::{Tera, Context};
use std::collections::HashMap;

use crate::config::Config;

pub fn setup_templates(config: &Config) -> Result<Tera> {
    let template_dir = config.template_dir
        .as_deref()
        .unwrap_or("py/janitor/site/templates");
    
    let template_pattern = format!("{}/**/*.html", template_dir);
    let mut tera = Tera::new(&template_pattern)?;
    
    // Register custom filters that match Python implementation
    tera.register_filter("basename", basename_filter);
    tera.register_filter("timeago", timeago_filter);
    tera.register_filter("duration", duration_filter);
    tera.register_filter("summarize", summarize_filter);
    
    // Register global functions
    tera.register_function("url_for", url_for_function);
    tera.register_function("get_flashed_messages", get_flashed_messages_function);
    
    Ok(tera)
}

// Custom filters to match Python Jinja2 implementation
fn basename_filter(value: &tera::Value, _: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    let path_str = value.as_str().ok_or_else(|| tera::Error::msg("Value must be a string"))?;
    let basename = std::path::Path::new(path_str)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path_str);
    Ok(tera::Value::String(basename.to_string()))
}

fn timeago_filter(value: &tera::Value, _: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    // TODO: Implement actual timeago formatting
    // For now, just return the value as-is
    Ok(value.clone())
}

fn duration_filter(value: &tera::Value, _: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    // TODO: Implement duration formatting
    // For now, just return the value as-is
    Ok(value.clone())
}

fn summarize_filter(value: &tera::Value, args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    let text = value.as_str().ok_or_else(|| tera::Error::msg("Value must be a string"))?;
    let length = args.get("length")
        .and_then(|v| v.as_u64())
        .unwrap_or(100) as usize;
    
    if text.len() <= length {
        Ok(value.clone())
    } else {
        let truncated = text.chars().take(length).collect::<String>();
        Ok(tera::Value::String(format!("{}...", truncated)))
    }
}

// Global functions to match Python implementation
fn url_for_function(args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    let endpoint = args.get("endpoint")
        .and_then(|v| v.as_str())
        .ok_or_else(|| tera::Error::msg("endpoint argument required"))?;
    
    // TODO: Implement actual URL generation based on routes
    // For now, return a placeholder
    Ok(tera::Value::String(format!("/{}", endpoint)))
}

fn get_flashed_messages_function(_args: &HashMap<String, tera::Value>) -> tera::Result<tera::Value> {
    // TODO: Implement flash message retrieval
    // For now, return empty array
    Ok(tera::Value::Array(vec![]))
}

pub fn create_base_context() -> Context {
    let mut context = Context::new();
    
    // Add global variables that are available in all templates
    context.insert("app_name", "Debian Janitor");
    context.insert("version", env!("CARGO_PKG_VERSION"));
    
    context
}