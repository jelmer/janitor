/// Module for scanning Debian package archives.
use deb822_lossless::FromDeb822Paragraph;
use debian_control::lossy::apt::{Package, Source};
use futures::stream::StreamExt;
use futures::stream::{self, Stream};
use futures::TryStreamExt;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;
use tracing::{debug, error, info, warn};

/// Scan binary packages in a directory.
///
/// # Arguments
/// * `td` - The directory to scan
/// * `arch` - Optional architecture to filter by
///
/// # Returns
/// A vector of Package objects or an error string
async fn scan_packages<'a>(td: &str, arch: Option<&str>) -> Result<Vec<Package>, String> {
    let mut args = Vec::new();
    if let Some(arch) = arch {
        args.extend(["-a", arch]);
    }

    let mut proc = Command::new("dpkg-scanpackages")
        .arg(td)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn dpkg-scanpackages process");

    let stdout = proc.stdout.take().expect("Failed to open stdout");
    let stderr = proc.stderr.take().expect("Failed to open stderr");

    let mut stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    let mut stdout = Vec::new();

    stdout_reader
        .read_to_end(&mut stdout)
        .await
        .map_err(|e| e.to_string())?;

    // Stream stdout paragraphs
    let paragraphs =
        deb822_lossless::lossy::Deb822::from_reader(&stdout[..]).map_err(|e| e.to_string())?;

    // Process stderr
    tokio::spawn(async move {
        let mut lines = stderr_reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.as_bytes();
            if line.starts_with(b"dpkg-scanpackages: ") {
                let line = &line[b"dpkg-scanpackages: ".len()..];
                handle_log_line(line);
            } else {
                handle_log_line(line);
            }
        }
    });

    paragraphs
        .into_iter()
        .map(|p| Package::from_paragraph(&p))
        .collect()
}

/// Scan source packages in a directory.
///
/// # Arguments
/// * `td` - The directory to scan
///
/// # Returns
/// A vector of Source objects or an error string
async fn scan_sources<'a>(td: &str) -> Result<Vec<Source>, String> {
    let mut proc = Command::new("dpkg-scansources")
        .arg(td)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn dpkg-scansources process");

    let stdout = proc.stdout.take().expect("Failed to open stdout");
    let stderr = proc.stderr.take().expect("Failed to open stderr");

    let mut stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    // Stream stdout paragraphs
    // TODO: Properly stream the output
    let mut stdout = Vec::new();

    stdout_reader
        .read_to_end(&mut stdout)
        .await
        .map_err(|e| e.to_string())?;

    let paragraphs =
        deb822_lossless::lossy::Deb822::from_reader(&stdout[..]).map_err(|e| e.to_string())?;

    // Process stderr
    tokio::spawn(async move {
        let mut lines = stderr_reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.as_bytes();
            if line.starts_with(b"dpkg-scansources: ") {
                let line = &line[b"dpkg-scansources: ".len()..];
                handle_log_line(line);
            } else {
                handle_log_line(line);
            }
        }
    });

    paragraphs
        .into_iter()
        .map(|p| Source::from_paragraph(&p))
        .collect()
}

/// Handle a log line from the scanner process.
///
/// # Arguments
/// * `line` - The log line as bytes
fn handle_log_line(line: &[u8]) {
    if line.starts_with(b"info: ") {
        debug!("{}", String::from_utf8_lossy(&line[b"info: ".len()..]));
    } else if line.starts_with(b"warning: ") {
        warn!("{}", String::from_utf8_lossy(&line[b"warning: ".len()..]));
    } else if line.starts_with(b"error: ") {
        error!("{}", String::from_utf8_lossy(&line[b"error: ".len()..]));
    } else {
        info!("dpkg error: {}", String::from_utf8_lossy(line));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scan_packages() {
        let packages = super::scan_packages("tests/data", None).await.unwrap();

        assert_eq!(packages.len(), 1);

        let package = &packages[0];

        assert_eq!(package.name, "hello");
        assert_eq!(package.version, "2.10-3".parse().unwrap());
    }

    #[tokio::test]
    async fn test_scan_sources() {
        let sources = super::scan_sources("tests/data").await.unwrap();

        assert_eq!(sources.len(), 1);

        let source = &sources[0];

        assert_eq!(source.package, "hello");
        assert_eq!(source.version, "2.10-3".parse().unwrap());
    }
}
