//! Package signing and upload functionality

use std::path::Path;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, error, info};

use crate::error::{Result, UploadError};
use crate::{DEBSIGN_FAILED_COUNT, UPLOAD_FAILED_COUNT};

/// Sign a Debian package using debsign
pub async fn sign_package(
    working_dir: &Path,
    changes_file: &str,
    keyid: Option<&str>,
) -> Result<()> {
    info!("Signing package: {}", changes_file);
    
    let mut cmd = Command::new("debsign");
    cmd.current_dir(working_dir)
        .arg("--no-conf")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    
    if let Some(key) = keyid {
        cmd.arg("-k").arg(key);
    }
    
    cmd.arg(changes_file);
    
    debug!("Running command: {:?}", cmd);
    
    let output = cmd.output().await?;
    
    if output.status.success() {
        info!("Successfully signed {}", changes_file);
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);
        
        error!(
            "Failed to sign {} (exit code {}): {}",
            changes_file, exit_code, stderr
        );
        
        DEBSIGN_FAILED_COUNT.inc();
        
        Err(UploadError::DebsignFailure(format!(
            "debsign failed with exit code {}: {}",
            exit_code, stderr
        )))
    }
}

/// Upload a Debian package using dput
pub async fn upload_package(
    working_dir: &Path,
    changes_file: &str,
    dput_host: &str,
) -> Result<()> {
    info!("Uploading package: {} to {}", changes_file, dput_host);
    
    let mut cmd = Command::new("dput");
    cmd.current_dir(working_dir)
        .arg("--no-upload-log")
        .arg("--unchecked")
        .arg(dput_host)
        .arg(changes_file)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    
    debug!("Running command: {:?}", cmd);
    
    let output = cmd.output().await?;
    
    if output.status.success() {
        info!("Successfully uploaded {} to {}", changes_file, dput_host);
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let exit_code = output.status.code().unwrap_or(-1);
        
        error!(
            "Failed to upload {} to {} (exit code {}): {} {}",
            changes_file, dput_host, exit_code, stdout, stderr
        );
        
        UPLOAD_FAILED_COUNT.inc();
        
        Err(UploadError::DputFailure(format!(
            "dput failed with exit code {}: {} {}",
            exit_code, stdout, stderr
        )))
    }
}

/// Check if debsign is available on the system
pub async fn check_debsign_available() -> bool {
    match Command::new("debsign")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
    {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

/// Check if dput is available on the system
pub async fn check_dput_available() -> bool {
    match Command::new("dput")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
    {
        Ok(status) => status.success(),
        Err(_) => false,
    }
}

/// Upload configuration
#[derive(Debug, Clone)]
pub struct UploadConfig {
    /// GPG key ID for signing
    pub debsign_keyid: Option<String>,
    /// dput host to upload to
    pub dput_host: String,
    /// Only upload source packages
    pub source_only: bool,
    /// Distributions to filter
    pub distributions: Vec<String>,
}

impl UploadConfig {
    /// Create a new upload configuration
    pub fn new(
        dput_host: String,
        debsign_keyid: Option<String>,
        source_only: bool,
        distributions: Vec<String>,
    ) -> Self {
        Self {
            debsign_keyid,
            dput_host,
            source_only,
            distributions,
        }
    }
    
    /// Check if a distribution should be uploaded
    pub fn should_upload_distribution(&self, distribution: &str) -> bool {
        if self.distributions.is_empty() {
            true
        } else {
            self.distributions.contains(&distribution.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_upload_config_distribution_filter() {
        let config = UploadConfig::new(
            "test-host".to_string(),
            None,
            false,
            vec!["unstable".to_string(), "experimental".to_string()],
        );
        
        assert!(config.should_upload_distribution("unstable"));
        assert!(config.should_upload_distribution("experimental"));
        assert!(!config.should_upload_distribution("stable"));
        
        // Empty distributions means upload all
        let config_all = UploadConfig::new(
            "test-host".to_string(),
            None,
            false,
            vec![],
        );
        
        assert!(config_all.should_upload_distribution("unstable"));
        assert!(config_all.should_upload_distribution("stable"));
        assert!(config_all.should_upload_distribution("any-dist"));
    }
}