use patchkit::unified::{iter_hunks, HunkLine};
use std::path::PathBuf;
use tracing::{debug, warn};

/// Output structure for diffoscope results.
///
/// This structure represents the JSON output from diffoscope and is used for
/// serialization, deserialization, and manipulation of diffoscope results.
#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq, Clone)]
#[allow(unused)]
pub struct DiffoscopeOutput {
    /// The version of the diffoscope JSON format.
    #[serde(
        rename = "diffoscope-json-version",
        skip_serializing_if = "Option::is_none"
    )]
    diffoscope_json_version: Option<u8>,
    /// The path to the first file being compared.
    pub source1: PathBuf,
    /// The path to the second file being compared.
    pub source2: PathBuf,
    /// Comments about the comparison, such as similarity percentage.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,
    /// The unified diff output, if available.
    #[serde(default)]
    pub unified_diff: Option<String>,
    /// Nested details for sub-comparisons of components within the files.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<DiffoscopeOutput>,
}

/// Implementation of the ToPyObject trait for DiffoscopeOutput.
///
/// This allows DiffoscopeOutput to be converted to a Python object,
/// which is necessary for interacting with Python diffoscope modules.
impl pyo3::ToPyObject for DiffoscopeOutput {
    /// Convert DiffoscopeOutput to a Python object.
    ///
    /// # Arguments
    /// * `py` - The Python interpreter
    ///
    /// # Returns
    /// A Python dictionary representing the DiffoscopeOutput
    fn to_object(&self, py: pyo3::Python) -> pyo3::PyObject {
        use pyo3::prelude::*;
        let dict = pyo3::types::PyDict::new_bound(py);
        dict.set_item("diffoscope_json_version", self.diffoscope_json_version)
            .unwrap();
        dict.set_item("source1", self.source1.to_str().unwrap())
            .unwrap();
        dict.set_item("source2", self.source2.to_str().unwrap())
            .unwrap();
        dict.set_item("comments", &self.comments).unwrap();
        dict.set_item("unified_diff", &self.unified_diff).unwrap();
        dict.set_item("details", &self.details).unwrap();
        dict.into()
    }
}

/// Errors that can occur when running diffoscope.
#[derive(Debug)]
pub enum DiffoscopeError {
    /// The diffoscope process timed out.
    Timeout,
    /// An I/O error occurred.
    Io(std::io::Error),
    /// An error occurred while parsing the JSON output.
    Serde(serde_json::Error),
    /// Any other error with a message.
    Other(String),
}

impl DiffoscopeError {
    /// Create a new generic error with a message.
    ///
    /// # Arguments
    /// * `msg` - The error message
    pub fn new(msg: &str) -> Self {
        DiffoscopeError::Other(msg.to_string())
    }
}

impl From<std::io::Error> for DiffoscopeError {
    fn from(err: std::io::Error) -> Self {
        DiffoscopeError::Io(err)
    }
}

impl From<serde_json::Error> for DiffoscopeError {
    fn from(err: serde_json::Error) -> Self {
        DiffoscopeError::Serde(err)
    }
}

impl std::fmt::Display for DiffoscopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            DiffoscopeError::Timeout => write!(f, "diffoscope timed out"),
            DiffoscopeError::Io(err) => write!(f, "IO error: {}", err),
            DiffoscopeError::Serde(err) => write!(f, "serde error: {}", err),
            DiffoscopeError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

/// Set comprehensive resource limits for the diffoscope process.
///
/// # Arguments
/// * `limit_mb` - Memory limit in megabytes
fn _set_limits(limit_mb: Option<u64>) {
    let limit_mb = limit_mb.unwrap_or(1024);
    let memory_bytes = limit_mb * 1024 * 1024;

    // Set virtual memory limit (RLIMIT_AS)
    if let Err(e) = nix::sys::resource::setrlimit(
        nix::sys::resource::Resource::RLIMIT_AS,
        memory_bytes,
        memory_bytes,
    ) {
        warn!("Failed to set RLIMIT_AS: {}", e);
    }

    // Set resident memory limit (RLIMIT_RSS) - physical memory
    // Note: RLIMIT_RSS is not available on all platforms (e.g., macOS)
    #[cfg(any(target_os = "linux", target_os = "android"))]
    if let Err(e) = nix::sys::resource::setrlimit(
        nix::sys::resource::Resource::RLIMIT_RSS,
        memory_bytes,
        memory_bytes,
    ) {
        warn!("Failed to set RLIMIT_RSS: {}", e);
    }

    // Set CPU time limit to prevent runaway processes (10 minutes)
    if let Err(e) = nix::sys::resource::setrlimit(
        nix::sys::resource::Resource::RLIMIT_CPU,
        600, // 10 minutes
        600,
    ) {
        warn!("Failed to set RLIMIT_CPU: {}", e);
    }

    // Set file descriptor limit
    if let Err(e) = nix::sys::resource::setrlimit(
        nix::sys::resource::Resource::RLIMIT_NOFILE,
        1024, // Max 1024 file descriptors
        1024,
    ) {
        warn!("Failed to set RLIMIT_NOFILE: {}", e);
    }

    debug!(
        "Set resource limits: memory={}MB, cpu=600s, fds=1024",
        limit_mb
    );
}

/// Run diffoscope on two binaries
///
/// # Arguments
/// * `old_binary` - The path to the old binary
/// * `new_binary` - The path to the new binary
/// * `diffoscope_command` - The command to run diffoscope
/// * `timeout` - The maximum time to run diffoscope
/// * `memory_limit` - The maximum memory to use
///
/// # Returns
/// * `Ok(Some(diffoscope_output))` - If diffoscope ran successfully
/// * `Ok(None)` - If diffoscope ran successfully but there were no differences
/// * `Err(DiffoscopeError)` - If there was an error running diffoscope
async fn _run_diffoscope(
    old_binary: &str,
    new_binary: &str,
    diffoscope_command: Option<&str>,
    timeout: Option<f64>,
    memory_limit: Option<usize>,
) -> Result<Option<DiffoscopeOutput>, DiffoscopeError> {
    let diffoscope_command = diffoscope_command.unwrap_or("diffoscope");
    let mut args = shlex::split(diffoscope_command).unwrap();
    args.extend([
        "--json=-".to_string(),
        "--exclude-directory-metadata=yes".to_string(),
        old_binary.to_string(),
        new_binary.to_string(),
    ]);

    let timeout_duration = std::time::Duration::from_secs_f64(timeout.unwrap_or(300.0)); // Default 5 minutes
    debug!(
        "Running diffoscope with timeout={:?}, memory_limit={:?}: {:?}",
        timeout_duration, memory_limit, args
    );

    let mut cmd = tokio::process::Command::new(&args[0]);
    cmd.args(&args[1..]);
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    // Set up process group for better cleanup
    cmd.process_group(0);

    if let Some(memory_limit) = memory_limit {
        let memory_limit_mb = memory_limit as u64;
        unsafe {
            cmd.pre_exec(move || {
                _set_limits(Some(memory_limit_mb));
                Ok(())
            })
        };
    }

    // Spawn the child process
    let child = cmd.spawn().map_err(DiffoscopeError::Io)?;
    let child_id = child.id();

    debug!("Started diffoscope process with PID: {:?}", child_id);

    // Wait for completion with timeout and cleanup
    let result =
        tokio::time::timeout(timeout_duration, async { child.wait_with_output().await }).await;

    let output = match result {
        Ok(Ok(output)) => {
            debug!("Diffoscope process completed normally");
            output
        }
        Ok(Err(e)) => {
            debug!("Diffoscope process failed: {}", e);
            return Err(DiffoscopeError::Io(e));
        }
        Err(_) => {
            // Timeout occurred, need to kill the process
            warn!(
                "Diffoscope process timed out (PID: {:?}), attempting cleanup",
                child_id
            );

            // Try to kill the entire process group to catch any spawned subprocesses
            if let Some(pid) = child_id {
                use nix::sys::signal::{self, Signal};
                use nix::unistd::Pid;

                if let Err(e) = signal::killpg(Pid::from_raw(pid as i32), Signal::SIGTERM) {
                    warn!("Failed to send SIGTERM to process group: {}", e);
                }

                // Wait a bit, then send SIGKILL if necessary
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                if let Err(e) = signal::killpg(Pid::from_raw(pid as i32), Signal::SIGKILL) {
                    warn!("Failed to send SIGKILL to process group: {}", e);
                }
            }

            return Err(DiffoscopeError::Timeout);
        }
    };

    // Check process completion status
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        debug!("Diffoscope stderr: {}", stderr);

        if output.status.code() == Some(1) {
            // Exit code 1 means differences found - this is expected
            let stdout_str = String::from_utf8_lossy(&output.stdout);
            match serde_json::from_str(&stdout_str) {
                Ok(result) => return Ok(Some(result)),
                Err(e) => {
                    warn!("Failed to parse diffoscope JSON output: {}", e);
                    return Err(DiffoscopeError::Serde(e));
                }
            }
        }

        // Other exit codes indicate errors
        if let Some(code) = output.status.code() {
            Err(DiffoscopeError::new(&format!(
                "Diffoscope failed with exit code {}: {}",
                code, stderr
            )))
        } else {
            Err(DiffoscopeError::new(&format!(
                "Diffoscope terminated by signal: {}",
                stderr
            )))
        }
    } else {
        // Exit code 0 means no differences found
        Ok(None)
    }
}

/// Filter out irrelevant information from the diff
/// (e.g. the full path to the binaries)
///
/// # Arguments
/// * `diff` - The diff to filter
pub fn filter_irrelevant(diff: &mut DiffoscopeOutput) {
    diff.source1 = diff.source1.file_name().unwrap().into();
    diff.source2 = diff.source2.file_name().unwrap().into();
}

/// Filter out boring information from the unified diff
///
/// This function replaces version-specific strings in the diff with a display version
/// to make the diff more readable and focused on actual changes.
///
/// # Arguments
/// * `udiff` - The unified diff to filter
/// * `old_version` - The old version string to replace
/// * `new_version` - The new version string to replace
/// * `display_version` - The version string to use in the output
///
/// # Returns
/// The filtered unified diff or an error
pub fn filter_boring_udiff(
    udiff: &str,
    old_version: &str,
    new_version: &str,
    display_version: &str,
) -> Result<String, patchkit::unified::Error> {
    let mut lines = udiff.lines().map(|line| line.as_bytes());
    let mut hunks = vec![];
    for hunk in iter_hunks(&mut lines) {
        let mut hunk = hunk?;
        for line in &mut hunk.lines {
            match line {
                HunkLine::RemoveLine(line) => {
                    *line = String::from_utf8(line.to_vec())
                        .unwrap()
                        .replace(old_version, display_version)
                        .into_bytes();
                }
                HunkLine::InsertLine(line) => {
                    *line = String::from_utf8(line.to_vec())
                        .unwrap()
                        .replace(new_version, display_version)
                        .into_bytes();
                }
                HunkLine::ContextLine(_line) => {}
            }
        }
        hunks.push(hunk);
    }
    Ok(hunks
        .iter()
        .map(|hunk| String::from_utf8(hunk.as_bytes()).unwrap())
        .collect())
}

/// Filter out boring details from a diffoscope output detail section
///
/// This function replaces version-specific strings in the detail section and filters
/// out uninteresting differences.
///
/// # Arguments
/// * `detail` - The detail section to filter
/// * `old_version` - The old version string to replace
/// * `new_version` - The new version string to replace
/// * `display_version` - The version string to use in the output
///
/// # Returns
/// `true` if the detail section still contains interesting differences, `false` otherwise
pub fn filter_boring_detail(
    detail: &mut DiffoscopeOutput,
    old_version: &str,
    new_version: &str,
    display_version: &str,
) -> bool {
    if let Some(unified_diff) = &detail.unified_diff {
        detail.unified_diff =
            match filter_boring_udiff(unified_diff, old_version, new_version, display_version) {
                Ok(udiff) => Some(udiff),
                Err(e) => {
                    warn!("Error parsing hunk: {:?}", e);
                    None
                }
            };
    }
    detail.source1 = detail
        .source1
        .to_str()
        .unwrap()
        .replace(old_version, display_version)
        .into();
    detail.source2 = detail
        .source2
        .to_str()
        .unwrap()
        .replace(new_version, display_version)
        .into();
    if !detail.details.is_empty() {
        let subdetails = detail.details.drain(..).filter_map(|mut subdetail| {
            if !filter_boring_detail(&mut subdetail, old_version, new_version, display_version) {
                return None;
            }
            Some(subdetail)
        });
        detail.details = subdetails.collect();
    }
    !(detail.unified_diff.is_none() && detail.details.is_empty())
}

/// Filter out boring information from the entire diffoscope output
///
/// This function filters out uninteresting differences from the entire diffoscope output,
/// such as changes in dates, distribution, and version.
///
/// # Arguments
/// * `diff` - The diffoscope output to filter
/// * `old_version` - The old version string
/// * `_old_campaign` - The old campaign string (unused)
/// * `new_version` - The new version string
/// * `_new_campaign` - The new campaign string (unused)
pub fn filter_boring(
    diff: &mut DiffoscopeOutput,
    old_version: &str,
    _old_campaign: &str,
    new_version: &str,
    _new_campaign: &str,
) {
    let display_version = new_version.rsplit_once("~").map_or(new_version, |(v, _)| v);
    // Changes file differences
    pub const BORING_FIELDS: &[&str] = &["Date", "Distribution", "Version"];
    let new_details = diff.details.drain(..).filter_map(|mut detail| {
        let boring = BORING_FIELDS.contains(&detail.source1.to_str().unwrap())
            && BORING_FIELDS.contains(&detail.source2.to_str().unwrap());
        if boring {
            return None;
        }
        if detail.source1.ends_with(".buildinfo") && detail.source2.ends_with(".buildinfo") {
            return None;
        }
        if !filter_boring_detail(&mut detail, old_version, new_version, display_version) {
            return None;
        }
        Some(detail)
    });
    diff.details = new_details.collect();
}

/// Format diffoscope output into various formats
///
/// This function converts diffoscope output into different formats like HTML, Markdown, plain text, or JSON.
///
/// # Arguments
/// * `diff` - The diffoscope output to format
/// * `content_type` - The desired output format ("text/html", "text/markdown", "text/plain", or "application/json")
/// * `title` - The title to use in the formatted output
/// * `css_url` - Optional URL for CSS styling (only used for HTML output)
///
/// # Returns
/// The formatted output as a string or a Python error
pub fn format_diffoscope(
    diff: &DiffoscopeOutput,
    content_type: &str,
    title: &str,
    css_url: Option<&str>,
) -> Result<String, pyo3::PyErr> {
    use pyo3::prelude::*;
    pyo3::prepare_freethreaded_python();
    if content_type == "application/json" {
        return Ok(serde_json::to_string(diff).unwrap());
    }

    Python::with_gil(|py| {
        let m = py.import_bound("diffoscope.readers.json")?;
        let reader = m.getattr("JSONReaderV1")?.call0()?;

        let root_differ = reader.call_method1("load_rec", (diff.to_object(py),))?;

        match content_type {
            "text/html" => {
                let m = py.import_bound("diffoscope.presenters.html")?;
                let p = m.getattr("HTMLPresenter")?.call0()?;

                let sysm = py.import_bound("sys")?;

                let old_stdout = sysm.getattr("stdout")?;
                let io = py.import_bound("io")?;
                let f = io.getattr("StringIO")?.call0()?;
                sysm.setattr("stdout", f.clone())?;
                let old_argv = sysm.getattr("argv")?;
                sysm.setattr(
                    "argv",
                    title.split(' ').map(|s| s.into()).collect::<Vec<String>>(),
                )?;

                let kwargs = pyo3::types::PyDict::new_bound(py);
                kwargs.set_item("css_url", css_url)?;
                p.call_method("output_html", ("-", root_differ), Some(&kwargs))?;
                let html = f.call_method0("getvalue")?;

                sysm.setattr("stdout", old_stdout)?;
                sysm.setattr("argv", old_argv)?;

                Ok(html.extract::<String>()?)
            }
            "text/markdown" => {
                let m = py.import_bound("diffoscope.presenters.markdown")?;
                let out = std::sync::Arc::new(pyo3::types::PyList::empty_bound(py).to_object(py));

                let println_out = out.clone();

                // Define a python callback that can take a string or no arguments
                // and append it to the out list
                let println = move |args: &Bound<pyo3::types::PyTuple>,
                                    _kwargs: Option<&Bound<pyo3::types::PyDict>>|
                      -> pyo3::PyResult<()> {
                    let s = if args.len() == 1 {
                        args.get_item(0).unwrap().extract::<String>()?
                    } else {
                        "".to_string()
                    };
                    Python::with_gil(|py| println_out.call_method1(py, "append", (s,)))?;
                    Ok(())
                };

                let pyprintln =
                    pyo3::types::PyCFunction::new_closure_bound(py, None, None, println)?;

                let presenter = m.getattr("MarkdownTextPresenter")?.call1((pyprintln,))?;
                presenter.call_method1("start", (root_differ,))?;
                Ok(out.extract::<Vec<String>>(py)?.join("\n"))
            }
            "text/plain" => {
                let m = py.import_bound("diffoscope.presenters.text")?;
                let out = pyo3::types::PyList::empty_bound(py);

                let presenter = m
                    .getattr("TextPresenter")?
                    .call1((out.getattr("append")?, false))?;
                presenter.call_method1("start", (root_differ,))?;

                Ok(out.extract::<Vec<String>>()?.join("\n"))
            }
            _ => Err(pyo3::exceptions::PyValueError::new_err(
                "Invalid content type",
            )),
        }
    })
}

/// Run diffoscope on two binaries
///
/// # Arguments
/// * `old_binaries` - A list of tuples containing the name and path of the old binaries
/// * `new_binaries` - A list of tuples containing the name and path of the new binaries
/// * `timeout` - The maximum time to run diffoscope
/// * `memory_limit` - The maximum memory to use
/// * `diffoscope_command` - The command to run diffoscope
pub async fn run_diffoscope(
    old_binaries: &[(&str, &str)],
    new_binaries: &[(&str, &str)],
    timeout: Option<f64>,
    memory_limit: Option<u64>,
    diffoscope_command: Option<&str>,
) -> Result<DiffoscopeOutput, DiffoscopeError> {
    let mut ret = DiffoscopeOutput {
        diffoscope_json_version: Some(1),
        source1: "old version".into(),
        source2: "new version".into(),
        comments: vec![],
        unified_diff: None,
        details: vec![],
    };

    for ((old_name, old_path), (new_name, new_path)) in old_binaries.iter().zip(new_binaries.iter())
    {
        let sub = _run_diffoscope(
            old_path,
            new_path,
            diffoscope_command,
            timeout,
            memory_limit.map(|mb| mb as usize),
        )
        .await?;
        if let Some(mut sub) = sub {
            sub.source1 = old_name.into();
            sub.source2 = new_name.into();
            sub.diffoscope_json_version = None;
            ret.details.push(sub);
        }
    }
    Ok(ret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run() {
        let td = tempfile::tempdir().unwrap();
        let old = td.path().join("old.json");
        let new = td.path().join("new.json");

        std::fs::write(&old, r#"{"foo": "bar"}"#).unwrap();
        std::fs::write(&new, r#"{"foo": "baz"}"#).unwrap();

        let diff = super::run_diffoscope(
            &[("old", old.to_str().unwrap())],
            &[("new", new.to_str().unwrap())],
            None,
            None,
            None,
        )
        .await
        .unwrap();

        assert_eq!(
            diff,
            DiffoscopeOutput {
                diffoscope_json_version: Some(1),
                source1: "old version".into(),
                source2: "new version".into(),
                comments: vec![],
                unified_diff: None,
                details: vec![DiffoscopeOutput {
                    diffoscope_json_version: None,
                    source1: "old".into(),
                    source2: "new".into(),
                    comments: vec![],
                    unified_diff: None,
                    details: vec![
                        DiffoscopeOutput {
                            diffoscope_json_version: None,
                            source1: "Pretty-printed".into(),
                            source2: "Pretty-printed".into(),
                            comments: vec!["Similarity: 0.5%".to_string(), "Differences: {\"'foo'\": \"'baz'\"}".to_string()],
                            unified_diff: Some("@@ -1,3 +1,3 @@\n {\n-    \"foo\": \"bar\"\n+    \"foo\": \"baz\"\n }\n".to_string()),
                            details: vec![]
                        }
                    ]

                }]
            }
        );
    }

    #[test]
    fn test_format_markdown() {
        let diff = DiffoscopeOutput {
            diffoscope_json_version: Some(1),
            source1: "old version".into(),
            source2: "new version".into(),
            comments: vec![],
            unified_diff: None,
            details: vec![DiffoscopeOutput {
                diffoscope_json_version: None,
                source1: "old".into(),
                source2: "new".into(),
                comments: vec![],
                unified_diff: Some(
                    "@@ -1,3 +1,3 @@\n {\n-    \"foo\": \"bar\"\n+    \"foo\": \"baz\"\n }\n"
                        .to_string(),
                ),
                details: vec![DiffoscopeOutput {
                    diffoscope_json_version: None,
                    source1: "Pretty-printed".into(),
                    source2: "Pretty-printed".into(),
                    comments: vec![
                        "Similarity: 0.5%".to_string(),
                        "Differences: {\"'foo'\": \"'baz'\"}".to_string(),
                    ],
                    unified_diff: Some(
                        "@@ -1,3 +1,3 @@\n {\n-    \"foo\": \"bar\"\n+    \"foo\": \"baz\"\n }\n"
                            .to_string(),
                    ),
                    details: vec![],
                }],
            }],
        };

        let markdown = format_diffoscope(&diff, "text/markdown", "title", None).unwrap();
        assert_eq!(markdown, "# Comparing `old version` & `new version`\n\n## Comparing `old` & `new`\n\n```diff\n@@ -1,3 +1,3 @@\n {\n-    \"foo\": \"bar\"\n+    \"foo\": \"baz\"\n }\n```\n\n### Pretty-printed\n\n * *Similarity: 0.5%*\n\n * *Differences: {\"'foo'\": \"'baz'\"}*\n\n```diff\n@@ -1,3 +1,3 @@\n {\n-    \"foo\": \"bar\"\n+    \"foo\": \"baz\"\n }\n```\n");
    }

    #[test]
    fn test_format_html() {
        let diff = DiffoscopeOutput {
            diffoscope_json_version: Some(1),
            source1: "old version".into(),
            source2: "new version".into(),
            comments: vec![],
            unified_diff: None,
            details: vec![DiffoscopeOutput {
                diffoscope_json_version: None,
                source1: "old".into(),
                source2: "new".into(),
                comments: vec![],
                unified_diff: Some(
                    "@@ -1,3 +1,3 @@\n {\n-    \"foo\": \"bar\"\n+    \"foo\": \"baz\"\n }\n"
                        .to_string(),
                ),
                details: vec![DiffoscopeOutput {
                    diffoscope_json_version: None,
                    source1: "Pretty-printed".into(),
                    source2: "Pretty-printed".into(),
                    comments: vec![
                        "Similarity: 0.5%".to_string(),
                        "Differences: {\"'foo'\": \"'baz'\"}".to_string(),
                    ],
                    unified_diff: Some(
                        "@@ -1,3 +1,3 @@\n {\n-    \"foo\": \"bar\"\n+    \"foo\": \"baz\"\n }\n"
                            .to_string(),
                    ),
                    details: vec![],
                }],
            }],
        };

        let html = format_diffoscope(&diff, "text/html", "title", None).unwrap();
        assert!(html.starts_with("<!DOCTYPE html>"));
    }

    #[test]
    fn test_format_json() {
        let diff = DiffoscopeOutput {
            diffoscope_json_version: Some(1),
            source1: "old version".into(),
            source2: "new version".into(),
            comments: vec![],
            unified_diff: None,
            details: vec![DiffoscopeOutput {
                diffoscope_json_version: None,
                source1: "old".into(),
                source2: "new".into(),
                comments: vec![],
                unified_diff: Some(
                    "@@ -1,3 +1,3 @@\n {\n-    \"foo\": \"bar\"\n+    \"foo\": \"baz\"\n }\n"
                        .to_string(),
                ),
                details: vec![DiffoscopeOutput {
                    diffoscope_json_version: None,
                    source1: "Pretty-printed".into(),
                    source2: "Pretty-printed".into(),
                    comments: vec![
                        "Similarity: 0.5%".to_string(),
                        "Differences: {\"'foo'\": \"'baz'\"}".to_string(),
                    ],
                    unified_diff: Some(
                        "@@ -1,3 +1,3 @@\n {\n-    \"foo\": \"bar\"\n+    \"foo\": \"baz\"\n }\n"
                            .to_string(),
                    ),
                    details: vec![],
                }],
            }],
        };

        let json = format_diffoscope(&diff, "application/json", "title", None).unwrap();
        assert_eq!(
            json,
            "{\"diffoscope-json-version\":1,\"source1\":\"old version\",\"source2\":\"new version\",\"unified_diff\":null,\"details\":[{\"source1\":\"old\",\"source2\":\"new\",\"unified_diff\":\"@@ -1,3 +1,3 @@\\n {\\n-    \\\"foo\\\": \\\"bar\\\"\\n+    \\\"foo\\\": \\\"baz\\\"\\n }\\n\",\"details\":[{\"source1\":\"Pretty-printed\",\"source2\":\"Pretty-printed\",\"comments\":[\"Similarity: 0.5%\",\"Differences: {\\\"'foo'\\\": \\\"'baz'\\\"}\"],\"unified_diff\":\"@@ -1,3 +1,3 @@\\n {\\n-    \\\"foo\\\": \\\"bar\\\"\\n+    \\\"foo\\\": \\\"baz\\\"\\n }\\n\"}]}]}"
        );
    }

    #[test]
    fn test_format_text() {
        let diff = DiffoscopeOutput {
            diffoscope_json_version: Some(1),
            source1: "old version".into(),
            source2: "new version".into(),
            comments: vec![],
            unified_diff: None,
            details: vec![DiffoscopeOutput {
                diffoscope_json_version: None,
                source1: "old".into(),
                source2: "new".into(),
                comments: vec![],
                unified_diff: Some(
                    "@@ -1,3 +1,3 @@\n {\n-    \"foo\": \"bar\"\n+    \"foo\": \"baz\"\n }\n"
                        .to_string(),
                ),
                details: vec![DiffoscopeOutput {
                    diffoscope_json_version: None,
                    source1: "Pretty-printed".into(),
                    source2: "Pretty-printed".into(),
                    comments: vec![
                        "Similarity: 0.5%".to_string(),
                        "Differences: {\"'foo'\": \"'baz'\"}".to_string(),
                    ],
                    unified_diff: Some(
                        "@@ -1,3 +1,3 @@\n {\n-    \"foo\": \"bar\"\n+    \"foo\": \"baz\"\n }\n"
                            .to_string(),
                    ),
                    details: vec![],
                }],
            }],
        };

        let text = format_diffoscope(&diff, "text/plain", "title", None).unwrap();
        assert_eq!(
            text,
            "--- old version\n+++ new version\n│   --- old\n├── +++ new\n│ @@ -1,3 +1,3 @@\n│  {\n│ -    \"foo\": \"bar\"\n│ +    \"foo\": \"baz\"\n│  }\n│ ├── Pretty-printed\n│ │┄ Similarity: 0.5%\n│ │┄ Differences: {\"'foo'\": \"'baz'\"}\n│ │ @@ -1,3 +1,3 @@\n│ │  {\n│ │ -    \"foo\": \"bar\"\n│ │ +    \"foo\": \"baz\"\n│ │  }"
        );
    }
}
