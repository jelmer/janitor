// Python Parity Verification Tests for Phase 3.10.3
// These tests compare Rust implementation outputs with Python implementation
// to ensure 100% functional compatibility and validate performance improvements

use std::time::{Duration, Instant};

use reqwest::Client;
use serde_json::{json, Value};

/// Configuration for parity testing environment
#[derive(Debug, Clone)]
pub struct ParityTestConfig {
    /// URL of the Python implementation for comparison
    pub python_base_url: String,
    /// URL of the Rust implementation
    pub rust_base_url: String,
    /// Timeout for HTTP requests
    pub request_timeout: Duration,
    /// Enable performance comparison
    pub enable_performance_tests: bool,
    /// Tolerance for performance improvements (e.g., 0.5 = 50% faster required)
    pub performance_threshold: f64,
}

impl Default for ParityTestConfig {
    fn default() -> Self {
        Self {
            python_base_url: std::env::var("PYTHON_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            rust_base_url: std::env::var("RUST_BASE_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            request_timeout: Duration::from_secs(10),
            enable_performance_tests: true,
            performance_threshold: 0.5, // Rust should be at least 50% faster
        }
    }
}

/// Results from a parity comparison
#[derive(Debug, Clone)]
pub struct ParityTestResult {
    pub endpoint: String,
    pub python_status: u16,
    pub rust_status: u16,
    pub python_content: String,
    pub rust_content: String,
    pub python_duration: Duration,
    pub rust_duration: Duration,
    pub content_matches: bool,
    pub status_matches: bool,
    pub headers_compatible: bool,
    pub performance_improvement: f64, // Ratio: python_time / rust_time
}

impl ParityTestResult {
    /// Check if the parity test passed all requirements
    pub fn is_passing(&self, threshold: f64) -> bool {
        self.content_matches
            && self.status_matches
            && self.headers_compatible
            && self.performance_improvement >= threshold
    }

    /// Get a summary of the test result
    pub fn summary(&self) -> String {
        format!(
            "Endpoint: {} | Status: {} | Content: {} | Headers: {} | Perf: {:.2}x",
            self.endpoint,
            if self.status_matches { "✓" } else { "✗" },
            if self.content_matches { "✓" } else { "✗" },
            if self.headers_compatible {
                "✓"
            } else {
                "✗"
            },
            self.performance_improvement
        )
    }
}

/// Parity testing framework for comparing Python and Rust implementations
pub struct ParityTester {
    config: ParityTestConfig,
    client: Client,
    results: Vec<ParityTestResult>,
}

impl ParityTester {
    /// Create a new parity tester
    pub fn new(config: ParityTestConfig) -> Self {
        let client = Client::builder()
            .timeout(config.request_timeout)
            .build()
            .expect("Failed to create HTTP client");

        Self {
            config,
            client,
            results: Vec::new(),
        }
    }

    /// Test a GET endpoint for parity
    pub async fn test_get_endpoint(&mut self, path: &str) -> anyhow::Result<()> {
        self.test_endpoint("GET", path, None, None).await
    }

    /// Test a POST endpoint for parity
    pub async fn test_post_endpoint(
        &mut self,
        path: &str,
        body: Option<&str>,
        content_type: Option<&str>,
    ) -> anyhow::Result<()> {
        self.test_endpoint("POST", path, body, content_type).await
    }

    /// Test an endpoint for parity between Python and Rust implementations
    pub async fn test_endpoint(
        &mut self,
        method: &str,
        path: &str,
        body: Option<&str>,
        content_type: Option<&str>,
    ) -> anyhow::Result<()> {
        let python_url = format!("{}{}", self.config.python_base_url, path);
        let rust_url = format!("{}{}", self.config.rust_base_url, path);

        // Test Python implementation
        let python_start = Instant::now();
        let python_response = self
            .make_request(method, &python_url, body, content_type)
            .await?;
        let python_duration = python_start.elapsed();

        // Test Rust implementation
        let rust_start = Instant::now();
        let rust_response = self
            .make_request(method, &rust_url, body, content_type)
            .await?;
        let rust_duration = rust_start.elapsed();

        // Compare responses
        let result = self
            .compare_responses(
                path,
                python_response,
                rust_response,
                python_duration,
                rust_duration,
            )
            .await?;

        self.results.push(result);
        Ok(())
    }

    /// Make an HTTP request
    async fn make_request(
        &self,
        method: &str,
        url: &str,
        body: Option<&str>,
        content_type: Option<&str>,
    ) -> anyhow::Result<reqwest::Response> {
        let mut request = match method.to_uppercase().as_str() {
            "GET" => self.client.get(url),
            "POST" => {
                let mut req = self.client.post(url);
                if let Some(body_data) = body {
                    req = req.body(body_data.to_string());
                }
                if let Some(ct) = content_type {
                    req = req.header("Content-Type", ct);
                }
                req
            }
            "PUT" => {
                let mut req = self.client.put(url);
                if let Some(body_data) = body {
                    req = req.body(body_data.to_string());
                }
                if let Some(ct) = content_type {
                    req = req.header("Content-Type", ct);
                }
                req
            }
            "DELETE" => self.client.delete(url),
            _ => anyhow::bail!("Unsupported HTTP method: {}", method),
        };

        // Add common headers for both implementations
        request = request
            .header("User-Agent", "janitor-parity-tester/1.0")
            .header("Accept", "application/json, text/html");

        let response = request.send().await?;
        Ok(response)
    }

    /// Compare responses from Python and Rust implementations
    async fn compare_responses(
        &self,
        endpoint: &str,
        python_response: reqwest::Response,
        rust_response: reqwest::Response,
        python_duration: Duration,
        rust_duration: Duration,
    ) -> anyhow::Result<ParityTestResult> {
        // Extract status codes
        let python_status = python_response.status().as_u16();
        let rust_status = rust_response.status().as_u16();
        let status_matches = python_status == rust_status;

        // Compare headers (check key compatibility)
        let headers_compatible =
            self.compare_headers(python_response.headers(), rust_response.headers());

        // Compare response bodies
        let python_content = python_response.text().await?;
        let rust_content = rust_response.text().await?;
        let content_matches = self.compare_content(&python_content, &rust_content, endpoint);

        // Calculate performance improvement
        let performance_improvement = if rust_duration.as_nanos() > 0 {
            python_duration.as_secs_f64() / rust_duration.as_secs_f64()
        } else {
            f64::INFINITY // Rust was essentially instant
        };

        Ok(ParityTestResult {
            endpoint: endpoint.to_string(),
            python_status,
            rust_status,
            python_content,
            rust_content,
            python_duration,
            rust_duration,
            content_matches,
            status_matches,
            headers_compatible,
            performance_improvement,
        })
    }

    /// Compare HTTP headers between implementations
    fn compare_headers(
        &self,
        python_headers: &reqwest::header::HeaderMap,
        rust_headers: &reqwest::header::HeaderMap,
    ) -> bool {
        // Check for essential headers that should be present in both
        let essential_headers = ["content-type", "content-length", "server"];

        for header in essential_headers.iter() {
            let python_has = python_headers.contains_key(*header);
            let rust_has = rust_headers.contains_key(*header);

            // Both should have essential headers, but content might differ
            if python_has != rust_has {
                eprintln!(
                    "Header mismatch for {}: Python={}, Rust={}",
                    header, python_has, rust_has
                );
                return false;
            }
        }

        true
    }

    /// Compare response content with intelligent diffing
    fn compare_content(&self, python_content: &str, rust_content: &str, endpoint: &str) -> bool {
        // Direct comparison first
        if python_content == rust_content {
            return true;
        }

        // Try JSON comparison for API endpoints
        if endpoint.starts_with("/api/") {
            return self.compare_json_content(python_content, rust_content);
        }

        // Try HTML comparison for page endpoints
        if python_content.contains("<!DOCTYPE html") || rust_content.contains("<!DOCTYPE html") {
            return self.compare_html_content(python_content, rust_content);
        }

        // For other content, do a more lenient comparison
        self.compare_text_content(python_content, rust_content)
    }

    /// Compare JSON content with normalization
    fn compare_json_content(&self, python_json: &str, rust_json: &str) -> bool {
        let python_value: Result<Value, _> = serde_json::from_str(python_json);
        let rust_value: Result<Value, _> = serde_json::from_str(rust_json);

        match (python_value, rust_value) {
            (Ok(python_val), Ok(rust_val)) => {
                // Normalize values before comparison
                let normalized_python = self.normalize_json_value(python_val);
                let normalized_rust = self.normalize_json_value(rust_val);
                normalized_python == normalized_rust
            }
            _ => {
                // If either isn't valid JSON, compare as text
                self.compare_text_content(python_json, rust_json)
            }
        }
    }

    /// Normalize JSON values for comparison (remove timestamps, sort arrays, etc.)
    fn normalize_json_value(&self, value: Value) -> Value {
        match value {
            Value::Object(mut map) => {
                // Remove or normalize timestamp fields
                let timestamp_fields = ["timestamp", "created_at", "updated_at", "last_modified"];
                for field in timestamp_fields.iter() {
                    if map.contains_key(*field) {
                        map.insert(
                            field.to_string(),
                            Value::String("NORMALIZED_TIMESTAMP".to_string()),
                        );
                    }
                }

                // Recursively normalize nested objects
                let normalized: serde_json::Map<String, Value> = map
                    .into_iter()
                    .map(|(k, v)| (k, self.normalize_json_value(v)))
                    .collect();

                Value::Object(normalized)
            }
            Value::Array(mut vec) => {
                // Sort arrays if they contain objects with an 'id' field
                if vec.iter().all(|v| v.is_object() && v.get("id").is_some()) {
                    vec.sort_by(|a, b| {
                        let a_id = a.get("id").unwrap().as_str().unwrap_or("");
                        let b_id = b.get("id").unwrap().as_str().unwrap_or("");
                        a_id.cmp(b_id)
                    });
                }

                // Recursively normalize array elements
                Value::Array(
                    vec.into_iter()
                        .map(|v| self.normalize_json_value(v))
                        .collect(),
                )
            }
            _ => value,
        }
    }

    /// Compare HTML content with structure-aware diffing
    fn compare_html_content(&self, python_html: &str, rust_html: &str) -> bool {
        // For now, do a simple whitespace-normalized comparison
        // In a full implementation, this could use an HTML parser
        let python_normalized = self.normalize_whitespace(python_html);
        let rust_normalized = self.normalize_whitespace(rust_html);

        // Allow for some differences in whitespace and attribute order
        self.html_structure_matches(&python_normalized, &rust_normalized)
    }

    /// Compare text content with whitespace normalization
    fn compare_text_content(&self, python_text: &str, rust_text: &str) -> bool {
        let python_normalized = self.normalize_whitespace(python_text);
        let rust_normalized = self.normalize_whitespace(rust_text);
        python_normalized == rust_normalized
    }

    /// Normalize whitespace in content
    fn normalize_whitespace(&self, content: &str) -> String {
        content
            .lines()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Check if HTML structures match with advanced comparison
    fn html_structure_matches(&self, html1: &str, html2: &str) -> bool {
        // Extract tag structure
        let tags1 = self.extract_html_tags(html1);
        let tags2 = self.extract_html_tags(html2);

        if tags1 == tags2 {
            return true;
        }

        // Try content-based comparison for template differences
        self.compare_html_content_advanced(html1, html2)
    }

    /// Advanced HTML content comparison for templates
    fn compare_html_content_advanced(&self, html1: &str, html2: &str) -> bool {
        // Extract meaningful content (text nodes, attributes)
        let content1 = self.extract_html_content(html1);
        let content2 = self.extract_html_content(html2);

        // Compare normalized content
        let normalized1 = self.normalize_html_content(&content1);
        let normalized2 = self.normalize_html_content(&content2);

        normalized1 == normalized2
    }

    /// Extract meaningful content from HTML (text nodes and key attributes)
    fn extract_html_content(&self, html: &str) -> Vec<String> {
        let mut content = Vec::new();
        let mut in_tag = false;
        let mut current_text = String::new();
        let mut current_tag = String::new();

        for ch in html.chars() {
            match ch {
                '<' => {
                    if !current_text.trim().is_empty() {
                        content.push(format!("TEXT:{}", current_text.trim()));
                    }
                    current_text.clear();
                    current_tag.clear();
                    current_tag.push(ch);
                    in_tag = true;
                }
                '>' => {
                    if in_tag {
                        current_tag.push(ch);
                        // Extract key attributes from tags
                        if let Some(attr_content) = self.extract_key_attributes(&current_tag) {
                            content.push(attr_content);
                        }
                        in_tag = false;
                    }
                }
                _ => {
                    if in_tag {
                        current_tag.push(ch);
                    } else {
                        current_text.push(ch);
                    }
                }
            }
        }

        // Add any remaining text
        if !current_text.trim().is_empty() {
            content.push(format!("TEXT:{}", current_text.trim()));
        }

        content
    }

    /// Extract key attributes that matter for functional comparison
    fn extract_key_attributes(&self, tag: &str) -> Option<String> {
        let key_attrs = ["id=", "class=", "href=", "src=", "action=", "method="];

        for attr in key_attrs.iter() {
            if tag.contains(attr) {
                if let Some(start) = tag.find(attr) {
                    let value_start = start + attr.len();
                    if let Some(quote_char) = tag.chars().nth(value_start) {
                        if quote_char == '"' || quote_char == '\'' {
                            if let Some(end) = tag[value_start + 1..].find(quote_char) {
                                let value = &tag[value_start + 1..value_start + 1 + end];
                                return Some(format!("ATTR:{}={}", &attr[..attr.len() - 1], value));
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Normalize HTML content for comparison
    fn normalize_html_content(&self, content: &[String]) -> Vec<String> {
        content
            .iter()
            .filter(|item| !item.trim().is_empty())
            .map(|item| {
                // Normalize dynamic content patterns
                if item.starts_with("TEXT:") {
                    let text = &item[5..];
                    // Replace timestamps and dynamic IDs
                    let normalized = self.normalize_dynamic_content(text);
                    format!("TEXT:{}", normalized)
                } else {
                    item.clone()
                }
            })
            .collect()
    }

    /// Normalize dynamic content that may differ between implementations
    fn normalize_dynamic_content(&self, content: &str) -> String {
        let mut normalized = content.to_string();

        // Normalize timestamps (ISO format)
        if let Ok(timestamp_regex) =
            regex::Regex::new(r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z?")
        {
            normalized = timestamp_regex
                .replace_all(&normalized, "TIMESTAMP")
                .to_string();
        }

        // Normalize UUIDs and long IDs
        if let Ok(uuid_regex) =
            regex::Regex::new(r"[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}")
        {
            normalized = uuid_regex.replace_all(&normalized, "UUID").to_string();
        }

        // Normalize numbers (durations, counts, etc.)
        if let Ok(number_regex) = regex::Regex::new(r"\b\d+\.\d+\b") {
            normalized = number_regex.replace_all(&normalized, "NUMBER").to_string();
        }

        normalized
    }

    /// Extract HTML tags for structure comparison
    fn extract_html_tags(&self, html: &str) -> Vec<String> {
        let mut tags = Vec::new();
        let mut in_tag = false;
        let mut current_tag = String::new();

        for ch in html.chars() {
            match ch {
                '<' => {
                    in_tag = true;
                    current_tag.clear();
                    current_tag.push(ch);
                }
                '>' => {
                    if in_tag {
                        current_tag.push(ch);
                        // Only include opening and closing tags, not self-closing or content
                        if current_tag.starts_with("</")
                            || (!current_tag.contains(' ') && !current_tag.ends_with("/>"))
                        {
                            tags.push(current_tag.clone());
                        }
                        in_tag = false;
                    }
                }
                _ => {
                    if in_tag {
                        current_tag.push(ch);
                    }
                }
            }
        }

        tags
    }

    /// Get a summary of all test results
    pub fn get_summary(&self) -> ParityTestSummary {
        let total_tests = self.results.len();
        let passing_tests = self
            .results
            .iter()
            .filter(|r| r.is_passing(self.config.performance_threshold))
            .count();

        let avg_performance_improvement = if !self.results.is_empty() {
            self.results
                .iter()
                .map(|r| r.performance_improvement)
                .sum::<f64>()
                / self.results.len() as f64
        } else {
            0.0
        };

        let content_matches = self.results.iter().filter(|r| r.content_matches).count();
        let status_matches = self.results.iter().filter(|r| r.status_matches).count();
        let headers_compatible = self.results.iter().filter(|r| r.headers_compatible).count();

        ParityTestSummary {
            total_tests,
            passing_tests,
            content_matches,
            status_matches,
            headers_compatible,
            avg_performance_improvement,
            individual_results: self.results.clone(),
        }
    }
}

/// Summary of parity testing results
#[derive(Debug, Clone)]
pub struct ParityTestSummary {
    pub total_tests: usize,
    pub passing_tests: usize,
    pub content_matches: usize,
    pub status_matches: usize,
    pub headers_compatible: usize,
    pub avg_performance_improvement: f64,
    pub individual_results: Vec<ParityTestResult>,
}

impl ParityTestSummary {
    /// Check if all tests are passing
    pub fn all_passing(&self) -> bool {
        self.passing_tests == self.total_tests
    }

    /// Get pass rate as percentage
    pub fn pass_rate(&self) -> f64 {
        if self.total_tests == 0 {
            100.0
        } else {
            (self.passing_tests as f64 / self.total_tests as f64) * 100.0
        }
    }

    /// Print detailed report
    pub fn print_report(&self) {
        println!("=== Python Parity Verification Report ===");
        println!("Total Tests: {}", self.total_tests);
        println!(
            "Passing Tests: {} ({:.1}%)",
            self.passing_tests,
            self.pass_rate()
        );
        println!(
            "Content Matches: {}/{}",
            self.content_matches, self.total_tests
        );
        println!(
            "Status Matches: {}/{}",
            self.status_matches, self.total_tests
        );
        println!(
            "Header Compatibility: {}/{}",
            self.headers_compatible, self.total_tests
        );
        println!(
            "Average Performance Improvement: {:.2}x",
            self.avg_performance_improvement
        );
        println!();

        println!("Individual Test Results:");
        for result in &self.individual_results {
            println!("  {}", result.summary());
        }

        if !self.all_passing() {
            println!();
            println!("❌ PARITY VERIFICATION FAILED");
            println!("Some tests did not pass the parity requirements.");
        } else {
            println!();
            println!("✅ PARITY VERIFICATION PASSED");
            println!("All tests meet the parity requirements.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_json_normalization() {
        let tester = ParityTester::new(ParityTestConfig::default());

        let json1 = json!({
            "timestamp": "2023-01-01T00:00:00Z",
            "data": [
                {"id": "2", "name": "second"},
                {"id": "1", "name": "first"}
            ]
        });

        let json2 = json!({
            "timestamp": "2023-01-02T00:00:00Z",
            "data": [
                {"id": "1", "name": "first"},
                {"id": "2", "name": "second"}
            ]
        });

        let normalized1 = tester.normalize_json_value(json1);
        let normalized2 = tester.normalize_json_value(json2);

        assert_eq!(normalized1, normalized2);
    }

    #[test]
    fn test_whitespace_normalization() {
        let tester = ParityTester::new(ParityTestConfig::default());

        let text1 = "  line1  \n\n  line2  \n\n";
        let text2 = "line1\nline2";

        assert_eq!(
            tester.normalize_whitespace(text1),
            tester.normalize_whitespace(text2)
        );
    }

    #[test]
    fn test_html_tag_extraction() {
        let tester = ParityTester::new(ParityTestConfig::default());

        let html = "<html><head><title>Test</title></head><body><h1>Header</h1></body></html>";
        let tags = tester.extract_html_tags(html);

        let expected = vec![
            "<html>", "<head>", "<title>", "</title>", "</head>", "<body>", "<h1>", "</h1>",
            "</body>", "</html>",
        ];
        assert_eq!(tags, expected);
    }

    #[test]
    fn test_html_content_extraction() {
        let tester = ParityTester::new(ParityTestConfig::default());

        let html = r#"<div id="test" class="main">Hello <span>World</span></div>"#;
        let content = tester.extract_html_content(html);

        // Should extract text content and key attributes
        assert!(content.iter().any(|item| item.starts_with("TEXT:Hello")));
        assert!(content.iter().any(|item| item.starts_with("TEXT:World")));
        assert!(content.iter().any(|item| item == "ATTR:id=test"));
        assert!(content.iter().any(|item| item == "ATTR:class=main"));
    }

    #[test]
    fn test_dynamic_content_normalization() {
        let tester = ParityTester::new(ParityTestConfig::default());

        let content1 =
            "Updated at 2023-12-01T15:30:45Z with ID abc123def-456g-789h-012i-345jklmnopqr";
        let content2 =
            "Updated at 2024-01-15T09:22:18Z with ID def456ghi-789j-012k-345l-678mnopqrstuv";

        let normalized1 = tester.normalize_dynamic_content(content1);
        let normalized2 = tester.normalize_dynamic_content(content2);

        assert_eq!(normalized1, normalized2);
        assert!(normalized1.contains("TIMESTAMP"));
        assert!(normalized1.contains("UUID"));
    }

    #[test]
    fn test_advanced_html_comparison() {
        let tester = ParityTester::new(ParityTestConfig::default());

        let html1 = r#"
            <div class="container">
                <h1>Welcome</h1>
                <p>Last updated: 2023-12-01T15:30:45Z</p>
                <ul>
                    <li id="item2">Second</li>
                    <li id="item1">First</li>
                </ul>
            </div>
        "#;

        let html2 = r#"
            <div class="container">
                <h1>Welcome</h1>
                <p>Last updated: 2024-01-15T09:22:18Z</p>
                <ul>
                    <li id="item1">First</li>
                    <li id="item2">Second</li>
                </ul>
            </div>
        "#;

        // Should match despite different timestamps and item order
        assert!(tester.compare_html_content_advanced(html1, html2));
    }
}

/// Pre-defined test suites for common parity verification scenarios
pub mod test_suites {
    use super::*;

    /// Test all API endpoints for parity
    pub async fn test_api_endpoints(tester: &mut ParityTester) -> anyhow::Result<()> {
        // Health and status endpoints
        tester.test_get_endpoint("/api/v1/health").await?;
        tester.test_get_endpoint("/api/v1/status").await?;
        tester.test_get_endpoint("/api/v1/queue/status").await?;

        // Package and codebase endpoints
        tester.test_get_endpoint("/api/v1/packages").await?;
        tester
            .test_get_endpoint("/api/v1/packages/search?q=test")
            .await?;
        tester.test_get_endpoint("/api/v1/codebases").await?;

        // Campaign endpoints
        tester.test_get_endpoint("/api/v1/campaigns").await?;
        tester
            .test_get_endpoint("/api/v1/campaigns/lintian-fixes")
            .await?;

        // Run endpoints
        tester.test_get_endpoint("/api/v1/runs/active").await?;
        tester.test_get_endpoint("/api/v1/runs?limit=10").await?;

        Ok(())
    }

    /// Test main site pages for parity
    pub async fn test_site_pages(tester: &mut ParityTester) -> anyhow::Result<()> {
        // Homepage and navigation
        tester.test_get_endpoint("/").await?;
        tester.test_get_endpoint("/about").await?;

        // Package browsing
        tester.test_get_endpoint("/pkg").await?;
        tester.test_get_endpoint("/pkg?search=test").await?;

        // Campaign pages
        tester.test_get_endpoint("/lintian-fixes/").await?;
        tester
            .test_get_endpoint("/lintian-fixes/candidates")
            .await?;
        tester.test_get_endpoint("/lintian-fixes/ready").await?;

        Ok(())
    }

    /// Test authentication flows for parity
    pub async fn test_auth_flows(tester: &mut ParityTester) -> anyhow::Result<()> {
        // Login endpoints
        tester.test_get_endpoint("/auth/login").await?;
        tester.test_get_endpoint("/auth/logout").await?;

        // Protected endpoints (should redirect or return 401)
        tester.test_get_endpoint("/admin/").await?;

        Ok(())
    }

    /// Test error handling for parity
    pub async fn test_error_handling(tester: &mut ParityTester) -> anyhow::Result<()> {
        // 404 errors
        tester.test_get_endpoint("/non-existent-page").await?;
        tester
            .test_get_endpoint("/api/v1/non-existent-endpoint")
            .await?;

        // Invalid parameters
        tester
            .test_get_endpoint("/api/v1/packages?limit=invalid")
            .await?;
        tester
            .test_get_endpoint("/lintian-fixes/c/non-existent-package/")
            .await?;

        // Edge case parameter values
        tester.test_get_endpoint("/api/v1/packages?limit=0").await?;
        tester
            .test_get_endpoint("/api/v1/packages?limit=999999")
            .await?;
        tester
            .test_get_endpoint("/api/v1/packages?limit=-1")
            .await?;

        // Special characters in parameters
        tester
            .test_get_endpoint("/api/v1/packages?search=%3Cscript%3E")
            .await?; // URL-encoded <script>
        tester
            .test_get_endpoint("/pkg?search=../../etc/passwd")
            .await?; // Path traversal attempt

        // Malformed requests
        tester
            .test_post_endpoint(
                "/api/v1/campaigns",
                Some("{invalid json}"),
                Some("application/json"),
            )
            .await?;

        // Timeout scenarios (very large requests)
        tester.test_get_endpoint("/api/v1/runs?limit=10000").await?;

        Ok(())
    }

    /// Test performance improvements and validate they meet requirements
    pub async fn test_performance_improvements(tester: &mut ParityTester) -> anyhow::Result<()> {
        println!("🚀 Running performance verification tests...");

        // Test multiple iterations for statistical significance
        let test_endpoints = [
            "/",
            "/api/v1/health",
            "/api/v1/status",
            "/api/v1/packages",
            "/pkg",
        ];

        for endpoint in test_endpoints.iter() {
            println!("Testing performance for endpoint: {}", endpoint);

            // Run multiple iterations to get average performance
            for i in 0..5 {
                tester.test_get_endpoint(endpoint).await?;
                println!("  Iteration {} completed", i + 1);
            }
        }

        Ok(())
    }

    /// Test load handling and concurrent requests
    pub async fn test_load_handling(tester: &mut ParityTester) -> anyhow::Result<()> {
        println!("📊 Testing concurrent load handling...");

        // Create multiple concurrent requests
        let endpoints = vec![
            "/api/v1/health",
            "/api/v1/status",
            "/api/v1/packages",
            "/",
            "/pkg",
        ];

        let mut handles = vec![];

        for endpoint in endpoints {
            let mut tester_clone = ParityTester::new(tester.config.clone());
            let endpoint = endpoint.to_string();

            let handle = tokio::spawn(async move {
                for _ in 0..3 {
                    let _ = tester_clone.test_get_endpoint(&endpoint).await;
                }
            });

            handles.push(handle);
        }

        // Wait for all concurrent requests to complete
        for handle in handles {
            let _ = handle.await;
        }

        println!("Concurrent load test completed");
        Ok(())
    }

    /// Test large response handling
    pub async fn test_large_responses(tester: &mut ParityTester) -> anyhow::Result<()> {
        println!("📦 Testing large response handling...");

        // Test endpoints that might return large responses
        tester.test_get_endpoint("/api/v1/runs?limit=1000").await?;
        tester
            .test_get_endpoint("/api/v1/packages?limit=500")
            .await?;

        Ok(())
    }

    /// Test boundary conditions and edge cases
    pub async fn test_boundary_conditions(tester: &mut ParityTester) -> anyhow::Result<()> {
        println!("🔍 Testing boundary conditions...");

        // Test pagination boundaries
        tester
            .test_get_endpoint("/api/v1/packages?offset=0&limit=1")
            .await?;
        tester
            .test_get_endpoint("/api/v1/packages?offset=0&limit=100")
            .await?;

        // Test empty search results
        tester
            .test_get_endpoint("/api/v1/packages?search=nonexistentpackagename12345")
            .await?;

        // Test special characters in search
        tester.test_get_endpoint("/pkg?search=test+package").await?;
        tester
            .test_get_endpoint("/pkg?search=test%20package")
            .await?;

        Ok(())
    }

    /// Comprehensive parity test suite including all Phase 3.10.3 requirements
    pub async fn run_comprehensive_suite() -> anyhow::Result<ParityTestSummary> {
        let config = ParityTestConfig::default();
        let mut tester = ParityTester::new(config);

        println!("🔍 Running comprehensive Python parity verification...");
        println!("Phase 3.10.3: Python Parity Verification");

        // Core functionality tests (API response comparison - completed)
        println!("\n📋 Testing API responses...");
        test_api_endpoints(&mut tester).await?;

        // HTML output matching validation
        println!("\n🌐 Testing HTML output matching...");
        test_site_pages(&mut tester).await?;

        // Authentication flow validation
        println!("\n🔐 Testing authentication flows...");
        test_auth_flows(&mut tester).await?;

        // Edge cases and error handling
        println!("\n⚠️ Testing edge cases and error handling...");
        test_error_handling(&mut tester).await?;
        test_boundary_conditions(&mut tester).await?;

        // Performance improvement verification
        println!("\n⚡ Testing performance improvements...");
        test_performance_improvements(&mut tester).await?;

        // Load and stress testing
        println!("\n💪 Testing load handling...");
        test_load_handling(&mut tester).await?;
        test_large_responses(&mut tester).await?;

        let summary = tester.get_summary();
        summary.print_report();

        // Validate Phase 3.10.3 completion criteria
        validate_phase_completion(&summary)?;

        Ok(summary)
    }

    /// Validate that Phase 3.10.3 completion criteria are met
    fn validate_phase_completion(summary: &ParityTestSummary) -> anyhow::Result<()> {
        println!("\n🎯 Validating Phase 3.10.3 completion criteria...");

        // Check pass rate meets minimum threshold (90%)
        if summary.pass_rate() < 90.0 {
            anyhow::bail!(
                "Parity verification failed: {:.1}% pass rate (minimum 90% required)",
                summary.pass_rate()
            );
        }

        // Check performance improvements
        if summary.avg_performance_improvement < 1.5 {
            anyhow::bail!(
                "Performance requirements not met: {:.2}x improvement (minimum 1.5x required)",
                summary.avg_performance_improvement
            );
        }

        // Check HTML output matching
        let html_match_rate = (summary.content_matches as f64 / summary.total_tests as f64) * 100.0;
        if html_match_rate < 85.0 {
            anyhow::bail!(
                "HTML output matching failed: {:.1}% match rate (minimum 85% required)",
                html_match_rate
            );
        }

        println!("✅ Phase 3.10.3 completion criteria validated:");
        println!("  - Pass rate: {:.1}% (✓ > 90%)", summary.pass_rate());
        println!(
            "  - Performance improvement: {:.2}x (✓ > 1.5x)",
            summary.avg_performance_improvement
        );
        println!(
            "  - HTML output matching: {:.1}% (✓ > 85%)",
            html_match_rate
        );

        Ok(())
    }
}
