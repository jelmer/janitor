use patchkit::unified::{iter_hunks, Hunk, HunkLine};
use std::path::PathBuf;
use tracing::{debug, warn};

#[derive(Debug, serde::Deserialize)]
#[allow(unused)]
pub struct DiffoscopeOutput {
    #[serde(
        rename = "diffoscope-json-version",
        skip_serializing_if = "Option::is_none"
    )]
    diffoscope_json_version: Option<u8>,
    source1: PathBuf,
    source2: PathBuf,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    comments: Vec<String>,
    #[serde(default)]
    unified_diff: Option<String>,
    details: Vec<DiffoscopeOutput>,
}

#[derive(Debug)]
pub enum DiffoscopeError {
    Timeout,
    Io(std::io::Error),
    Serde(serde_json::Error),
    Other(String),
}

impl DiffoscopeError {
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

async fn _run_diffoscope(
    old_binary: &str,
    new_binary: &str,
    diffoscope_command: Option<&str>,
    timeout: Option<f64>,
) -> Result<Option<DiffoscopeOutput>, DiffoscopeError> {
    let diffoscope_command = diffoscope_command.unwrap_or("diffoscope");
    let mut args = shlex::split(diffoscope_command).unwrap();
    args.extend([
        "--json=-".to_string(),
        "--exclude-directory-metadata=yes".to_string(),
        old_binary.to_string(),
        new_binary.to_string(),
    ]);
    debug!("running {:?}", args);

    let mut cmd = tokio::process::Command::new(&args[0]);
    cmd.args(&args[1..]);
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let output = tokio::time::timeout(
        std::time::Duration::from_secs_f64(timeout.unwrap_or(5.0)),
        cmd.output(),
    )
    .await
    .map_err(|_| DiffoscopeError::Timeout)?
    .map_err(DiffoscopeError::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if output.status.code() == Some(1) {
            return Ok(Some(serde_json::from_str(&String::from_utf8_lossy(
                &output.stdout,
            ))?));
        }
        Err(DiffoscopeError::new(&stderr))
    } else {
        Ok(None)
    }
}

pub fn filter_irrelevant(diff: &mut DiffoscopeOutput) {
    diff.source1 = diff.source1.file_name().unwrap().into();
    diff.source2 = diff.source2.file_name().unwrap().into();
}

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

pub fn filter_boring(
    diff: &mut DiffoscopeOutput,
    old_version: &str,
    old_campaign: &str,
    new_version: &str,
    new_campaign: &str,
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

pub async fn run_diffoscope(
    old_binaries: &[(String, String)],
    new_binaries: &[(String, String)],
    timeout: Option<f64>,
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
        let sub = _run_diffoscope(old_path, new_path, diffoscope_command, timeout).await?;
        if let Some(mut sub) = sub {
            sub.source1 = old_name.into();
            sub.source2 = new_name.into();
            sub.diffoscope_json_version = None;
            ret.details.push(sub);
        }
    }
    Ok(ret)
}
