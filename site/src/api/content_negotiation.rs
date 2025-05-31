use axum::{
    extract::Request,
    http::{header, HeaderMap, HeaderValue},
    response::{Html, IntoResponse, Response},
    Json,
};
use serde::Serialize;
use std::str::FromStr;
use tracing::{debug, warn};

/// Supported content types for API responses
#[derive(Debug, Clone, PartialEq)]
pub enum ContentType {
    Json,
    Html,
    TextPlain,
    TextDiff,
    OctetStream,
    Xml,
    Csv,
}

impl ContentType {
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Json => "application/json",
            Self::Html => "text/html",
            Self::TextPlain => "text/plain",
            Self::TextDiff => "text/x-diff",
            Self::OctetStream => "application/octet-stream",
            Self::Xml => "application/xml",
            Self::Csv => "text/csv",
        }
    }

    pub fn file_extension(&self) -> Option<&'static str> {
        match self {
            Self::Json => Some("json"),
            Self::Html => Some("html"),
            Self::TextPlain => Some("txt"),
            Self::TextDiff => Some("diff"),
            Self::OctetStream => None,
            Self::Xml => Some("xml"),
            Self::Csv => Some("csv"),
        }
    }
}

impl FromStr for ContentType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "application/json" | "json" => Ok(Self::Json),
            "text/html" | "html" => Ok(Self::Html),
            "text/plain" | "plain" => Ok(Self::TextPlain),
            "text/x-diff" | "diff" => Ok(Self::TextDiff),
            "application/octet-stream" | "binary" => Ok(Self::OctetStream),
            "application/xml" | "xml" => Ok(Self::Xml),
            "text/csv" | "csv" => Ok(Self::Csv),
            _ => Err(()),
        }
    }
}

/// Content negotiation based on Accept header and URL extensions
pub fn negotiate_content_type(headers: &HeaderMap, path: &str) -> ContentType {
    // First, check for file extension in path
    if let Some(extension) = extract_file_extension(path) {
        match extension {
            "json" => return ContentType::Json,
            "html" => return ContentType::Html,
            "txt" => return ContentType::TextPlain,
            "diff" => return ContentType::TextDiff,
            _ => {}
        }
    }

    // Then check Accept header
    if let Some(accept_header) = headers.get(header::ACCEPT) {
        if let Ok(accept_str) = accept_header.to_str() {
            return parse_accept_header(accept_str);
        }
    }

    // Default to JSON for API endpoints, HTML for others
    if path.starts_with("/api/") || path.starts_with("/cupboard/api/") {
        ContentType::Json
    } else {
        ContentType::Html
    }
}

/// Parse Accept header and find best matching content type
fn parse_accept_header(accept: &str) -> ContentType {
    let mut best_match = ContentType::Json;
    let mut best_quality = 0.0;

    // Simple Accept header parsing (not fully RFC compliant but practical)
    for part in accept.split(',') {
        let part = part.trim();
        let (media_type, quality) = parse_media_type(part);
        
        let content_type = match media_type.as_str() {
            "application/json" => ContentType::Json,
            "text/html" => ContentType::Html,
            "text/plain" => ContentType::TextPlain,
            "text/x-diff" => ContentType::TextDiff,
            "application/octet-stream" => ContentType::OctetStream,
            "application/xml" => ContentType::Xml,
            "text/csv" => ContentType::Csv,
            "*/*" => ContentType::Json, // Default for wildcard
            _ if media_type.starts_with("text/") => ContentType::TextPlain,
            _ if media_type.starts_with("application/") => ContentType::Json,
            _ => continue,
        };

        if quality > best_quality {
            best_match = content_type;
            best_quality = quality;
        }
    }

    debug!("Content negotiation: Accept='{}' -> {:?}", accept, best_match);
    best_match
}

/// Parse individual media type with quality factor
fn parse_media_type(media_type: &str) -> (String, f64) {
    if let Some((media, params)) = media_type.split_once(';') {
        let media = media.trim().to_lowercase();
        
        // Look for q= parameter
        for param in params.split(';') {
            let param = param.trim();
            if let Some((key, value)) = param.split_once('=') {
                if key.trim() == "q" {
                    if let Ok(quality) = value.trim().parse::<f64>() {
                        return (media, quality.clamp(0.0, 1.0));
                    }
                }
            }
        }
        
        (media, 1.0)
    } else {
        (media_type.trim().to_lowercase(), 1.0)
    }
}

/// Extract file extension from URL path
fn extract_file_extension(path: &str) -> Option<&str> {
    // Remove query parameters
    let path = path.split('?').next().unwrap_or(path);
    
    // Get filename part
    let filename = path.split('/').last()?;
    
    // Extract extension
    if let Some((_, extension)) = filename.rsplit_once('.') {
        Some(extension)
    } else {
        None
    }
}

/// Wrapper for responses that can be rendered in multiple formats
#[derive(Debug)]
pub struct NegotiatedResponse<T> {
    pub data: T,
    pub content_type: ContentType,
    pub template_name: Option<String>,
}

impl<T> NegotiatedResponse<T> {
    pub fn new(data: T, content_type: ContentType) -> Self {
        Self {
            data,
            content_type,
            template_name: None,
        }
    }

    pub fn with_template(mut self, template_name: String) -> Self {
        self.template_name = Some(template_name);
        self
    }
}

impl<T> IntoResponse for NegotiatedResponse<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        match self.content_type {
            ContentType::Json => {
                let mut response = Json(self.data).into_response();
                response.headers_mut().insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                );
                response
            }
            ContentType::Html => {
                // For HTML responses, we'd typically render with a template engine
                // For now, we'll serialize to JSON and wrap in basic HTML
                match serde_json::to_string_pretty(&self.data) {
                    Ok(json) => {
                        let html = if let Some(template) = self.template_name {
                            format!(
                                r#"<!DOCTYPE html>
<html>
<head>
    <title>{}</title>
    <style>pre {{ background: #f5f5f5; padding: 1em; }}</style>
</head>
<body>
    <h1>{}</h1>
    <pre>{}</pre>
</body>
</html>"#,
                                template, template, json
                            )
                        } else {
                            format!(
                                r#"<!DOCTYPE html>
<html>
<head>
    <title>API Response</title>
    <style>pre {{ background: #f5f5f5; padding: 1em; }}</style>
</head>
<body>
    <pre>{}</pre>
</body>
</html>"#,
                                json
                            )
                        };
                        Html(html).into_response()
                    }
                    Err(_) => {
                        warn!("Failed to serialize data for HTML response");
                        Html("<html><body><h1>Error</h1><p>Failed to render response</p></body></html>").into_response()
                    }
                }
            }
            ContentType::TextPlain => {
                match serde_json::to_string_pretty(&self.data) {
                    Ok(json) => {
                        let mut response = json.into_response();
                        response.headers_mut().insert(
                            header::CONTENT_TYPE,
                            HeaderValue::from_static("text/plain"),
                        );
                        response
                    }
                    Err(_) => {
                        warn!("Failed to serialize data for plain text response");
                        "Error: Failed to render response".into_response()
                    }
                }
            }
            ContentType::TextDiff => {
                // For diff responses, we expect the data to be a string
                match serde_json::to_string(&self.data) {
                    Ok(content) => {
                        let mut response = content.into_response();
                        response.headers_mut().insert(
                            header::CONTENT_TYPE,
                            HeaderValue::from_static("text/x-diff"),
                        );
                        response
                    }
                    Err(_) => {
                        warn!("Failed to serialize data for diff response");
                        "Error: Failed to render diff".into_response()
                    }
                }
            }
            ContentType::OctetStream => {
                // For binary responses, assume data is bytes or can be converted
                match serde_json::to_vec(&self.data) {
                    Ok(bytes) => {
                        let mut response = bytes.into_response();
                        response.headers_mut().insert(
                            header::CONTENT_TYPE,
                            HeaderValue::from_static("application/octet-stream"),
                        );
                        response
                    }
                    Err(_) => {
                        warn!("Failed to serialize data for binary response");
                        "Error: Failed to render binary response".into_response()
                    }
                }
            }
            ContentType::Xml => {
                // For XML responses, serialize to JSON and return as XML
                match serde_json::to_string_pretty(&self.data) {
                    Ok(json) => {
                        let xml = format!(r#"<?xml version="1.0" encoding="UTF-8"?><data>{}</data>"#, json);
                        let mut response = xml.into_response();
                        response.headers_mut().insert(
                            header::CONTENT_TYPE,
                            HeaderValue::from_static("application/xml"),
                        );
                        response
                    }
                    Err(_) => {
                        warn!("Failed to serialize data for XML response");
                        "Error: Failed to render XML response".into_response()
                    }
                }
            }
            ContentType::Csv => {
                // For CSV responses, attempt to serialize as CSV
                match serde_json::to_string(&self.data) {
                    Ok(content) => {
                        let mut response = content.into_response();
                        response.headers_mut().insert(
                            header::CONTENT_TYPE,
                            HeaderValue::from_static("text/csv"),
                        );
                        response
                    }
                    Err(_) => {
                        warn!("Failed to serialize data for CSV response");
                        "Error: Failed to render CSV response".into_response()
                    }
                }
            }
        }
    }
}

/// Helper function to create negotiated responses
pub fn negotiate_response<T>(
    data: T,
    headers: &HeaderMap,
    path: &str,
) -> NegotiatedResponse<T> {
    let content_type = negotiate_content_type(headers, path);
    NegotiatedResponse::new(data, content_type)
}

/// Helper function to create negotiated responses with template
pub fn negotiate_response_with_template<T>(
    data: T,
    headers: &HeaderMap,
    path: &str,
    template_name: String,
) -> NegotiatedResponse<T> {
    let content_type = negotiate_content_type(headers, path);
    NegotiatedResponse::new(data, content_type).with_template(template_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn test_content_type_mime_types() {
        assert_eq!(ContentType::Json.mime_type(), "application/json");
        assert_eq!(ContentType::Html.mime_type(), "text/html");
        assert_eq!(ContentType::TextDiff.mime_type(), "text/x-diff");
    }

    #[test]
    fn test_extract_file_extension() {
        assert_eq!(extract_file_extension("/api/test.json"), Some("json"));
        assert_eq!(extract_file_extension("/api/test.html"), Some("html"));
        assert_eq!(extract_file_extension("/api/test"), None);
        assert_eq!(extract_file_extension("/api/test.json?param=value"), Some("json"));
    }

    #[test]
    fn test_parse_media_type() {
        assert_eq!(parse_media_type("application/json"), ("application/json".to_string(), 1.0));
        assert_eq!(parse_media_type("text/html; q=0.8"), ("text/html".to_string(), 0.8));
        assert_eq!(parse_media_type("*/*; q=0.1"), ("*/*".to_string(), 0.1));
    }

    #[test]
    fn test_negotiate_content_type() {
        let mut headers = HeaderMap::new();
        
        // Test default for API paths
        assert_eq!(negotiate_content_type(&headers, "/api/test"), ContentType::Json);
        assert_eq!(negotiate_content_type(&headers, "/test"), ContentType::Html);
        
        // Test file extensions
        assert_eq!(negotiate_content_type(&headers, "/api/test.json"), ContentType::Json);
        assert_eq!(negotiate_content_type(&headers, "/api/test.html"), ContentType::Html);
        
        // Test Accept header
        headers.insert(header::ACCEPT, "application/json".parse().unwrap());
        assert_eq!(negotiate_content_type(&headers, "/test"), ContentType::Json);
        
        headers.insert(header::ACCEPT, "text/html".parse().unwrap());
        assert_eq!(negotiate_content_type(&headers, "/api/test"), ContentType::Html);
    }

    #[test]
    fn test_parse_accept_header() {
        assert_eq!(parse_accept_header("application/json"), ContentType::Json);
        assert_eq!(parse_accept_header("text/html"), ContentType::Html);
        assert_eq!(parse_accept_header("text/html, application/json; q=0.8"), ContentType::Html);
        assert_eq!(parse_accept_header("application/json; q=0.9, text/html; q=0.8"), ContentType::Json);
    }
}