use anyhow::Result;
use axum::{
    body::Body,
    http::{header, HeaderValue},
    response::Response,
};
use std::collections::HashMap;
use std::path::Path;
use tower_http::services::ServeDir;
use tracing::{debug, warn};

/// Asset management for static files with compression and caching
#[derive(Debug, Clone)]
pub struct AssetManager {
    /// Base directory for static assets
    asset_dir: String,
    /// Asset fingerprints for cache busting
    fingerprints: HashMap<String, String>,
    /// Enable compression for assets
    enable_compression: bool,
    /// Cache control headers
    cache_max_age: u32,
}

impl AssetManager {
    /// Create a new asset manager
    pub fn new(asset_dir: String) -> Self {
        Self {
            asset_dir,
            fingerprints: HashMap::new(),
            enable_compression: true,
            cache_max_age: 31536000, // 1 year for static assets
        }
    }

    /// Create asset manager with development settings (no caching)
    pub fn development(asset_dir: String) -> Self {
        Self {
            asset_dir,
            fingerprints: HashMap::new(),
            enable_compression: false,
            cache_max_age: 0, // No caching in development
        }
    }

    /// Get the URL for an asset with optional fingerprinting
    pub fn asset_url(&self, path: &str) -> String {
        if let Some(fingerprint) = self.fingerprints.get(path) {
            format!("/_static/{}?v={}", path, fingerprint)
        } else {
            format!("/_static/{}", path)
        }
    }

    /// Get CSS URL for a stylesheet
    pub fn css_url(&self, name: &str) -> String {
        self.asset_url(&format!("css/{}", name))
    }

    /// Get JavaScript URL for a script
    pub fn js_url(&self, name: &str) -> String {
        self.asset_url(&format!("js/{}", name))
    }

    /// Get image URL for an image
    pub fn img_url(&self, name: &str) -> String {
        self.asset_url(&format!("img/{}", name))
    }

    /// Generate asset fingerprints for cache busting
    pub fn generate_fingerprints(&mut self) -> Result<()> {
        let asset_dir = self.asset_dir.clone();
        let asset_path = Path::new(&asset_dir);
        if !asset_path.exists() {
            warn!("Asset directory does not exist: {}", asset_dir);
            return Ok(());
        }

        self.scan_directory(asset_path, "")?;
        debug!("Generated {} asset fingerprints", self.fingerprints.len());
        Ok(())
    }

    /// Recursively scan directory for assets
    fn scan_directory(&mut self, dir: &Path, prefix: &str) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();

            if path.is_dir() {
                let new_prefix = if prefix.is_empty() {
                    file_name
                } else {
                    format!("{}/{}", prefix, file_name)
                };
                self.scan_directory(&path, &new_prefix)?;
            } else {
                let relative_path = if prefix.is_empty() {
                    file_name
                } else {
                    format!("{}/{}", prefix, file_name)
                };

                // Generate fingerprint based on file content hash
                if let Ok(content) = std::fs::read(&path) {
                    let fingerprint = self.generate_content_hash(&content);
                    self.fingerprints.insert(relative_path, fingerprint);
                }
            }
        }
        Ok(())
    }

    /// Generate a simple content hash for fingerprinting
    fn generate_content_hash(&self, content: &[u8]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Create a service for serving static files with proper headers
    pub fn create_static_service(&self) -> ServeDir {
        ServeDir::new(&self.asset_dir)
            .precompressed_gzip()
            .precompressed_br()
    }

    /// Add appropriate headers for static assets
    pub fn add_static_headers(
        &self,
        mut response: Response<Body>,
        file_path: &str,
    ) -> Response<Body> {
        let headers = response.headers_mut();

        // Add cache control headers
        if self.cache_max_age > 0 {
            headers.insert(
                header::CACHE_CONTROL,
                HeaderValue::from_str(&format!("public, max-age={}", self.cache_max_age))
                    .unwrap_or_else(|_| HeaderValue::from_static("public, max-age=31536000")),
            );
        }

        // Add content type based on file extension
        if let Some(content_type) = self.get_content_type(file_path) {
            headers.insert(header::CONTENT_TYPE, content_type);
        }

        // Add compression headers if enabled
        if self.enable_compression {
            if let Some(encoding) = self.get_content_encoding(file_path) {
                headers.insert(header::CONTENT_ENCODING, encoding);
            }
        }

        response
    }

    /// Get content type based on file extension
    fn get_content_type(&self, file_path: &str) -> Option<HeaderValue> {
        let extension = Path::new(file_path).extension()?.to_str()?.to_lowercase();

        let content_type = match extension.as_str() {
            "css" => "text/css; charset=utf-8",
            "js" => "application/javascript; charset=utf-8",
            "json" => "application/json; charset=utf-8",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "gif" => "image/gif",
            "svg" => "image/svg+xml; charset=utf-8",
            "ico" => "image/x-icon",
            "woff" => "font/woff",
            "woff2" => "font/woff2",
            "ttf" => "font/ttf",
            "eot" => "application/vnd.ms-fontobject",
            "html" => "text/html; charset=utf-8",
            "txt" => "text/plain; charset=utf-8",
            _ => return None,
        };

        HeaderValue::from_str(content_type).ok()
    }

    /// Get content encoding for compressed files
    fn get_content_encoding(&self, file_path: &str) -> Option<HeaderValue> {
        if file_path.ends_with(".gz") {
            HeaderValue::from_static("gzip").into()
        } else if file_path.ends_with(".br") {
            HeaderValue::from_static("br").into()
        } else {
            None
        }
    }
}

/// Asset manifest for template context
#[derive(Debug, Clone)]
pub struct AssetManifest {
    manager: AssetManager,
}

impl AssetManifest {
    /// Create asset manifest from manager
    pub fn new(manager: AssetManager) -> Self {
        Self { manager }
    }

    /// Get CSS URL for templates
    pub fn css(&self, name: &str) -> String {
        self.manager.css_url(name)
    }

    /// Get JavaScript URL for templates
    pub fn js(&self, name: &str) -> String {
        self.manager.js_url(name)
    }

    /// Get image URL for templates
    pub fn img(&self, name: &str) -> String {
        self.manager.img_url(name)
    }

    /// Get generic asset URL for templates
    pub fn asset(&self, path: &str) -> String {
        self.manager.asset_url(path)
    }
}

/// Development asset watcher for hot reload
pub struct AssetWatcher {
    asset_dir: String,
    manager: AssetManager,
    should_stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl AssetWatcher {
    /// Create a new asset watcher
    pub fn new(asset_dir: String) -> Self {
        let manager = AssetManager::development(asset_dir.clone());
        Self {
            asset_dir,
            manager,
            should_stop: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Start watching for asset changes
    pub async fn start(&self) -> Result<()> {
        use std::path::Path;
        use std::time::{Duration, SystemTime};

        debug!("Asset watcher started for directory: {}", self.asset_dir);
        
        let asset_path = Path::new(&self.asset_dir);
        if !asset_path.exists() {
            warn!("Asset directory does not exist: {}", self.asset_dir);
            return Ok(());
        }

        // Keep track of file modification times
        let mut file_timestamps = std::collections::HashMap::new();
        
        // Initial scan
        self.scan_files(&asset_path, &mut file_timestamps)?;
        
        // Watch for changes every 1 second in development
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        
        while !self.should_stop.load(std::sync::atomic::Ordering::Relaxed) {
            interval.tick().await;
            
            if let Err(e) = self.check_for_changes(&asset_path, &mut file_timestamps).await {
                warn!("Error checking for asset changes: {}", e);
            }
        }
        
        debug!("Asset watcher stopped");
        Ok(())
    }

    /// Stop the asset watcher
    pub fn stop(&self) {
        self.should_stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Scan all files in the directory and record their timestamps
    fn scan_files(
        &self,
        dir: &Path,
        file_timestamps: &mut std::collections::HashMap<std::path::PathBuf, std::time::SystemTime>,
    ) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                self.scan_files(&path, file_timestamps)?;
            } else if self.is_watchable_file(&path) {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        file_timestamps.insert(path, modified);
                    }
                }
            }
        }
        Ok(())
    }

    /// Check if a file should be watched for changes
    fn is_watchable_file(&self, path: &Path) -> bool {
        if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
            matches!(
                extension.to_lowercase().as_str(),
                "css" | "js" | "html" | "svg" | "json" | "png" | "jpg" | "jpeg" | "gif" | "ico" | "woff" | "woff2" | "ttf"
            )
        } else {
            false
        }
    }

    /// Check for file changes and trigger reprocessing
    async fn check_for_changes(
        &self,
        dir: &Path,
        file_timestamps: &mut std::collections::HashMap<std::path::PathBuf, std::time::SystemTime>,
    ) -> Result<()> {
        let mut changes_detected = false;
        
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                Box::pin(self.check_for_changes(&path, file_timestamps)).await?;
            } else if self.is_watchable_file(&path) {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if let Some(&previous_time) = file_timestamps.get(&path) {
                            if modified > previous_time {
                                debug!("Asset changed: {}", path.display());
                                file_timestamps.insert(path.clone(), modified);
                                self.handle_file_change(&path).await?;
                                changes_detected = true;
                            }
                        } else {
                            // New file
                            debug!("New asset detected: {}", path.display());
                            file_timestamps.insert(path.clone(), modified);
                            self.handle_file_change(&path).await?;
                            changes_detected = true;
                        }
                    }
                }
            }
        }
        
        // Check for deleted files
        let existing_files: std::collections::HashSet<_> = file_timestamps.keys().cloned().collect();
        let mut files_to_remove = Vec::new();
        
        for file_path in &existing_files {
            if !file_path.exists() {
                debug!("Asset deleted: {}", file_path.display());
                files_to_remove.push(file_path.clone());
                changes_detected = true;
            }
        }
        
        for file_path in files_to_remove {
            file_timestamps.remove(&file_path);
        }
        
        if changes_detected {
            debug!("Asset changes detected, regenerating fingerprints");
            // In a real implementation, this would trigger browser refresh
            // For now, just log the change
        }
        
        Ok(())
    }

    /// Handle a single file change
    async fn handle_file_change(&self, file_path: &Path) -> Result<()> {
        debug!("Processing changed file: {}", file_path.display());
        
        // Check if this is a CSS or JS file that should be optimized
        if let Some(extension) = file_path.extension().and_then(|s| s.to_str()) {
            match extension.to_lowercase().as_str() {
                "css" => {
                    if let Ok(content) = std::fs::read_to_string(file_path) {
                        let optimized = AssetOptimizer::optimize_css(&content);
                        let optimized_path = file_path.with_extension("min.css");
                        if let Err(e) = std::fs::write(&optimized_path, optimized) {
                            warn!("Failed to write optimized CSS to {}: {}", optimized_path.display(), e);
                        } else {
                            debug!("Generated optimized CSS: {}", optimized_path.display());
                        }
                    }
                }
                "js" => {
                    if let Ok(content) = std::fs::read_to_string(file_path) {
                        let optimized = AssetOptimizer::optimize_js(&content);
                        let optimized_path = file_path.with_extension("min.js");
                        if let Err(e) = std::fs::write(&optimized_path, optimized) {
                            warn!("Failed to write optimized JS to {}: {}", optimized_path.display(), e);
                        } else {
                            debug!("Generated optimized JS: {}", optimized_path.display());
                        }
                    }
                }
                _ => {} // Other file types don't need optimization
            }
        }
        
        // Compress the file if it should be compressed
        if AssetOptimizer::should_compress(file_path) {
            if let Err(e) = AssetOptimizer::compress_file(file_path) {
                warn!("Failed to compress asset {}: {}", file_path.display(), e);
            }
        }
        
        Ok(())
    }
}

/// Asset optimization utilities
pub struct AssetOptimizer;

impl AssetOptimizer {
    /// Optimize CSS files by removing comments and unnecessary whitespace
    pub fn optimize_css(content: &str) -> String {
        let mut optimized = String::with_capacity(content.len());
        let mut in_comment = false;
        let mut in_string = false;
        let mut string_delimiter = '\0';
        let mut chars = content.chars().peekable();
        let mut last_char = ' ';

        while let Some(ch) = chars.next() {
            match ch {
                // Handle string literals
                '"' | '\'' if !in_comment => {
                    if !in_string {
                        in_string = true;
                        string_delimiter = ch;
                        optimized.push(ch);
                    } else if ch == string_delimiter && last_char != '\\' {
                        in_string = false;
                        optimized.push(ch);
                    } else {
                        optimized.push(ch);
                    }
                }
                // Handle comments
                '/' if !in_string && !in_comment => {
                    if chars.peek() == Some(&'*') {
                        in_comment = true;
                        chars.next(); // consume '*'
                    } else {
                        optimized.push(ch);
                    }
                }
                '*' if in_comment && !in_string => {
                    if chars.peek() == Some(&'/') {
                        in_comment = false;
                        chars.next(); // consume '/'
                    }
                }
                // Handle whitespace
                ' ' | '\t' | '\r' | '\n' if !in_string && !in_comment => {
                    // Only add whitespace if the last character wasn't whitespace
                    // and if it's needed for separation
                    if !last_char.is_whitespace() && Self::needs_whitespace(last_char, chars.peek().copied()) {
                        optimized.push(' ');
                        last_char = ' ';
                    }
                }
                // Handle other characters
                _ if !in_comment => {
                    optimized.push(ch);
                    last_char = ch;
                }
                _ => {} // Skip characters inside comments
            }
        }

        optimized.trim().to_string()
    }

    /// Check if whitespace is needed between two characters
    fn needs_whitespace(last: char, next: Option<char>) -> bool {
        if let Some(next_char) = next {
            // Keep whitespace between alphanumeric characters
            if last.is_alphanumeric() && next_char.is_alphanumeric() {
                return true;
            }
            // Keep whitespace around certain operators
            if matches!(last, '+' | '-' | '*' | '/' | '=' | '<' | '>') && next_char.is_alphanumeric() {
                return true;
            }
            if last.is_alphanumeric() && matches!(next_char, '+' | '-' | '*' | '/' | '=' | '<' | '>') {
                return true;
            }
        }
        false
    }

    /// Optimize JavaScript files by removing comments and unnecessary whitespace
    pub fn optimize_js(content: &str) -> String {
        let mut optimized = String::with_capacity(content.len());
        let mut in_single_comment = false;
        let mut in_multi_comment = false;
        let mut in_string = false;
        let mut string_delimiter = '\0';
        let mut chars = content.chars().peekable();
        let mut last_char = ' ';

        while let Some(ch) = chars.next() {
            match ch {
                // Handle string literals
                '"' | '\'' | '`' if !in_single_comment && !in_multi_comment => {
                    if !in_string {
                        in_string = true;
                        string_delimiter = ch;
                        optimized.push(ch);
                    } else if ch == string_delimiter && last_char != '\\' {
                        in_string = false;
                        optimized.push(ch);
                    } else {
                        optimized.push(ch);
                    }
                }
                // Handle single-line comments
                '/' if !in_string && !in_multi_comment => {
                    if chars.peek() == Some(&'/') {
                        in_single_comment = true;
                        chars.next(); // consume second '/'
                    } else if chars.peek() == Some(&'*') && !in_single_comment {
                        in_multi_comment = true;
                        chars.next(); // consume '*'
                    } else {
                        optimized.push(ch);
                    }
                }
                // Handle multi-line comments
                '*' if in_multi_comment && !in_string => {
                    if chars.peek() == Some(&'/') {
                        in_multi_comment = false;
                        chars.next(); // consume '/'
                    }
                }
                // Handle newlines
                '\n' if in_single_comment => {
                    in_single_comment = false;
                    // Add a space instead of newline to maintain separation
                    if !last_char.is_whitespace() {
                        optimized.push(' ');
                        last_char = ' ';
                    }
                }
                // Handle whitespace
                ' ' | '\t' | '\r' | '\n' if !in_string && !in_single_comment && !in_multi_comment => {
                    // Only add whitespace if needed for separation
                    if !last_char.is_whitespace() && Self::needs_js_whitespace(last_char, chars.peek().copied()) {
                        optimized.push(' ');
                        last_char = ' ';
                    }
                }
                // Handle other characters
                _ if !in_single_comment && !in_multi_comment => {
                    optimized.push(ch);
                    last_char = ch;
                }
                _ => {} // Skip characters inside comments
            }
        }

        optimized.trim().to_string()
    }

    /// Check if whitespace is needed between two characters in JavaScript
    fn needs_js_whitespace(last: char, next: Option<char>) -> bool {
        if let Some(next_char) = next {
            // Keep whitespace between keywords and identifiers
            if last.is_alphanumeric() && next_char.is_alphanumeric() {
                return true;
            }
            if last == '_' && next_char.is_alphanumeric() {
                return true;
            }
            if last.is_alphanumeric() && next_char == '_' {
                return true;
            }
            // Keep whitespace around certain operators
            if matches!(last, '+' | '-' | '=' | '<' | '>' | '!' | '&' | '|') && next_char.is_alphanumeric() {
                return true;
            }
            if last.is_alphanumeric() && matches!(next_char, '+' | '-' | '=' | '<' | '>' | '!' | '&' | '|') {
                return true;
            }
        }
        false
    }

    /// Generate compressed versions of assets
    pub fn compress_assets(asset_dir: &str) -> Result<()> {
        let asset_path = Path::new(asset_dir);
        if !asset_path.exists() {
            return Ok(());
        }

        Self::compress_directory(asset_path)?;
        debug!("Compressed assets in {}", asset_dir);
        Ok(())
    }

    /// Recursively compress files in directory
    fn compress_directory(dir: &Path) -> Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                Self::compress_directory(&path)?;
            } else if Self::should_compress(&path) {
                Self::compress_file(&path)?;
            }
        }
        Ok(())
    }

    /// Check if file should be compressed
    fn should_compress(path: &Path) -> bool {
        if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
            matches!(
                extension.to_lowercase().as_str(),
                "css" | "js" | "html" | "svg" | "json"
            )
        } else {
            false
        }
    }

    /// Compress a single file using gzip
    fn compress_file(path: &Path) -> Result<()> {
        use std::fs::File;
        use std::io::{Read, Write};

        let mut file = File::open(path)?;
        let mut content = Vec::new();
        file.read_to_end(&mut content)?;

        // Create gzip compressed version
        let gzip_path = path.with_extension(format!(
            "{}.gz",
            path.extension().unwrap_or_default().to_string_lossy()
        ));

        let gzip_file = File::create(gzip_path)?;
        let mut encoder = flate2::write::GzEncoder::new(gzip_file, flate2::Compression::default());
        encoder.write_all(&content)?;
        encoder.finish()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_manager_creation() {
        let manager = AssetManager::new("static".to_string());
        assert_eq!(manager.asset_dir, "static");
        assert!(manager.enable_compression);
        assert_eq!(manager.cache_max_age, 31536000);
    }

    #[test]
    fn test_development_asset_manager() {
        let manager = AssetManager::development("static".to_string());
        assert_eq!(manager.asset_dir, "static");
        assert!(!manager.enable_compression);
        assert_eq!(manager.cache_max_age, 0);
    }

    #[test]
    fn test_asset_url_generation() {
        let manager = AssetManager::new("static".to_string());
        assert_eq!(manager.css_url("style.css"), "/_static/css/style.css");
        assert_eq!(manager.js_url("app.js"), "/_static/js/app.js");
        assert_eq!(manager.img_url("logo.png"), "/_static/img/logo.png");
    }

    #[test]
    fn test_content_type_detection() {
        let manager = AssetManager::new("static".to_string());

        assert_eq!(
            manager.get_content_type("style.css").unwrap(),
            HeaderValue::from_static("text/css; charset=utf-8")
        );

        assert_eq!(
            manager.get_content_type("app.js").unwrap(),
            HeaderValue::from_static("application/javascript; charset=utf-8")
        );

        assert_eq!(
            manager.get_content_type("image.png").unwrap(),
            HeaderValue::from_static("image/png")
        );
    }

    #[test]
    fn test_asset_manifest() {
        let manager = AssetManager::new("static".to_string());
        let manifest = AssetManifest::new(manager);

        assert_eq!(manifest.css("main.css"), "/_static/css/main.css");
        assert_eq!(manifest.js("app.js"), "/_static/js/app.js");
        assert_eq!(
            manifest.asset("fonts/icon.woff"),
            "/_static/fonts/icon.woff"
        );
    }

    #[test]
    fn test_css_optimization() {
        let css_input = r#"
        /* This is a comment */
        .class {
            color: red;  /* another comment */
            margin: 0   ;
        }
        
        /* Multi-line
           comment */
        .another-class { background: blue; }
        "#;

        let optimized = AssetOptimizer::optimize_css(css_input);
        
        // Should remove comments and unnecessary whitespace
        assert!(!optimized.contains("/*"));
        assert!(!optimized.contains("*/"));
        assert!(optimized.contains(".class"));
        assert!(optimized.contains("color:red"));
        assert!(optimized.contains("margin:0"));
    }

    #[test]
    fn test_js_optimization() {
        let js_input = r#"
        // Single line comment
        function test() {
            /* Multi-line comment */
            var x = 1  +  2;  // Another comment
            return x;
        }
        
        // Function call
        test();
        "#;

        let optimized = AssetOptimizer::optimize_js(js_input);
        
        // Should remove comments and unnecessary whitespace
        assert!(!optimized.contains("//"));
        assert!(!optimized.contains("/*"));
        assert!(!optimized.contains("*/"));
        assert!(optimized.contains("function test()"));
        assert!(optimized.contains("var x = 1 + 2"));
        assert!(optimized.contains("test()"));
    }

    #[test]
    fn test_css_string_preservation() {
        let css_input = r#"
        .class {
            content: "/* not a comment */";
            background: url('http://example.com/image.png');
        }
        "#;

        let optimized = AssetOptimizer::optimize_css(css_input);
        
        // Should preserve strings
        assert!(optimized.contains("\"/* not a comment */\""));
        assert!(optimized.contains("'http://example.com/image.png'"));
    }

    #[test]
    fn test_js_string_preservation() {
        let js_input = r#"
        var str = "// not a comment";
        var str2 = '/* also not a comment */';
        var template = `// template ${variable} comment`;
        "#;

        let optimized = AssetOptimizer::optimize_js(js_input);
        
        // Should preserve strings
        assert!(optimized.contains("\"// not a comment\""));
        assert!(optimized.contains("'/* also not a comment */'"));
        assert!(optimized.contains("`// template ${variable} comment`"));
    }

    #[test]
    fn test_asset_watcher_creation() {
        let watcher = AssetWatcher::new("static".to_string());
        assert_eq!(watcher.asset_dir, "static");
        assert!(!watcher.should_stop.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn test_watchable_file_detection() {
        let watcher = AssetWatcher::new("static".to_string());
        
        assert!(watcher.is_watchable_file(std::path::Path::new("style.css")));
        assert!(watcher.is_watchable_file(std::path::Path::new("app.js")));
        assert!(watcher.is_watchable_file(std::path::Path::new("image.png")));
        assert!(watcher.is_watchable_file(std::path::Path::new("font.woff2")));
        
        assert!(!watcher.is_watchable_file(std::path::Path::new("readme.txt")));
        assert!(!watcher.is_watchable_file(std::path::Path::new("config.toml")));
        assert!(!watcher.is_watchable_file(std::path::Path::new("script.py")));
    }
}
