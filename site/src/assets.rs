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

/// Development asset watcher for hot reload (placeholder)
pub struct AssetWatcher {
    _asset_dir: String,
}

impl AssetWatcher {
    /// Create a new asset watcher
    pub fn new(asset_dir: String) -> Self {
        Self {
            _asset_dir: asset_dir,
        }
    }

    /// Start watching for asset changes (placeholder for development)
    pub async fn start(&self) -> Result<()> {
        // In a full implementation, this would use a file watcher
        // like `notify` crate to watch for file changes
        debug!("Asset watcher started (placeholder implementation)");
        Ok(())
    }
}

/// Asset optimization utilities
pub struct AssetOptimizer;

impl AssetOptimizer {
    /// Optimize CSS files (placeholder)
    pub fn optimize_css(content: &str) -> String {
        // In a full implementation, this would:
        // - Remove comments and whitespace
        // - Merge duplicate rules
        // - Optimize selectors
        content.to_string()
    }

    /// Optimize JavaScript files (placeholder)
    pub fn optimize_js(content: &str) -> String {
        // In a full implementation, this would:
        // - Minify the JavaScript
        // - Remove dead code
        // - Optimize for size
        content.to_string()
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
}
