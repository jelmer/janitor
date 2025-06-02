use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tera::Context;

use crate::auth::User;

/// Common template context data that's included in most pages
#[derive(Debug, Serialize, Deserialize)]
pub struct BaseContext {
    pub site_name: String,
    pub site_url: String,
    pub current_url: String,
    pub user: Option<User>,
    pub is_admin: bool,
    pub is_qa_reviewer: bool,
    pub analytics_enabled: bool,
    pub flash_messages: Vec<FlashMessage>,
    pub layout: String,
}

impl BaseContext {
    pub fn new(site_name: String, site_url: String, current_url: String) -> Self {
        Self {
            site_name,
            site_url,
            current_url,
            user: None,
            is_admin: false,
            is_qa_reviewer: false,
            analytics_enabled: false,
            flash_messages: Vec::new(),
            layout: "default".to_string(),
        }
    }

    pub fn with_user(mut self, user: Option<User>) -> Self {
        if let Some(ref u) = user {
            self.is_admin = u.is_admin();
            self.is_qa_reviewer = u.is_qa_reviewer();
        }
        self.user = user;
        self
    }

    pub fn with_flash_messages(mut self, messages: Vec<FlashMessage>) -> Self {
        self.flash_messages = messages;
        self
    }

    pub fn with_layout(mut self, layout: &str) -> Self {
        self.layout = layout.to_string();
        self
    }

    /// Convert to Tera context
    pub fn to_context(&self) -> Context {
        Context::from_serialize(self).expect("Failed to serialize base context")
    }

    /// Merge with existing context
    pub fn merge_into(&self, context: &mut Context) {
        context.insert("site_name", &self.site_name);
        context.insert("site_url", &self.site_url);
        context.insert("current_url", &self.current_url);
        context.insert("user", &self.user);
        context.insert("is_admin", &self.is_admin);
        context.insert("is_qa_reviewer", &self.is_qa_reviewer);
        context.insert("analytics_enabled", &self.analytics_enabled);
        context.insert("flash_messages", &self.flash_messages);
        context.insert("layout", &self.layout);
    }
}

/// Flash message for user notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashMessage {
    pub category: String,
    pub message: String,
}

impl FlashMessage {
    pub fn success(message: impl Into<String>) -> Self {
        Self {
            category: "success".to_string(),
            message: message.into(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            category: "error".to_string(),
            message: message.into(),
        }
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self {
            category: "warning".to_string(),
            message: message.into(),
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self {
            category: "info".to_string(),
            message: message.into(),
        }
    }
}

/// Template-specific context builders
pub struct ContextBuilder;

impl ContextBuilder {
    /// Build context for error pages
    pub fn error_context(
        base: BaseContext,
        error_code: u16,
        error_message: &str,
        error_details: Option<&str>,
    ) -> Context {
        let mut context = base.to_context();
        context.insert("error_code", &error_code);
        context.insert("error_message", error_message);
        context.insert("error_details", &error_details);
        context
    }

    /// Build context for suite pages
    pub fn suite_context(
        base: BaseContext,
        suite: &str,
        suite_description: &str,
        active_page: &str,
    ) -> Context {
        let mut context = base.to_context();
        context.insert("suite", suite);
        context.insert("suite_description", suite_description);
        context.insert("active_page", active_page);
        context
    }

    /// Build context for package/codebase pages
    pub fn codebase_context(
        base: BaseContext,
        codebase: &str,
        suite: &str,
        runs: Vec<serde_json::Value>,
    ) -> Context {
        let mut context = base.to_context();
        context.insert("codebase", codebase);
        context.insert("suite", suite);
        context.insert("runs", &runs);
        context
    }

    /// Build context for admin pages
    pub fn admin_context(base: BaseContext, active_page: &str, data: serde_json::Value) -> Context {
        let mut context = base.with_layout("admin").to_context();
        context.insert("active_page", active_page);
        context.insert("data", &data);
        context
    }
}

/// Helper functions for template rendering
pub fn add_common_helpers(context: &mut Context, request_path: &str) {
    // Add commonly used template variables
    context.insert("now", &chrono::Utc::now());
    context.insert("request_path", request_path);

    // Add build information
    context.insert("version", env!("CARGO_PKG_VERSION"));
    context.insert("build_time", option_env!("BUILD_TIME").unwrap_or("unknown"));
    context.insert(
        "git_revision",
        option_env!("GIT_REVISION").unwrap_or("unknown"),
    );
}

/// Template cache key builder for efficient caching
pub fn build_cache_key(template_name: &str, params: &HashMap<String, String>) -> String {
    let mut parts = vec![template_name.to_string()];

    // Sort parameters for consistent cache keys
    let mut sorted_params: Vec<_> = params.iter().collect();
    sorted_params.sort_by_key(|(k, _)| k.as_str());

    for (key, value) in sorted_params {
        parts.push(format!("{}={}", key, value));
    }

    parts.join(":")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base_context() {
        let base = BaseContext::new(
            "Test Site".to_string(),
            "https://example.com".to_string(),
            "/test".to_string(),
        );

        let context = base.to_context();
        assert_eq!(
            context.get("site_name").and_then(|v| v.as_str()),
            Some("Test Site")
        );
    }

    #[test]
    fn test_flash_messages() {
        let messages = vec![
            FlashMessage::success("Operation completed"),
            FlashMessage::error("Something went wrong"),
        ];

        assert_eq!(messages[0].category, "success");
        assert_eq!(messages[1].category, "error");
    }

    #[test]
    fn test_cache_key_builder() {
        let mut params = HashMap::new();
        params.insert("b".to_string(), "2".to_string());
        params.insert("a".to_string(), "1".to_string());

        let key = build_cache_key("template.html", &params);
        assert_eq!(key, "template.html:a=1:b=2");
    }
}
