use tracing::debug;

#[derive(Debug, serde::Deserialize)]
struct DiffoscopeOutput {
    #[serde(
        rename = "diffoscope-json-version",
        skip_serializing_if = "Option::is_none"
    )]
    diffoscope_json_version: Option<u8>,
    source1: String,
    source2: String,
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

    let mut output = tokio::time::timeout(
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
        return Err(DiffoscopeError::new(&stderr));
    } else {
        return Ok(None);
    }
}

pub async fn run_diffoscope(
    old_binaries: &[(String, String)],
    new_binaries: &[(String, String)],
    timeout: Option<f64>,
    diffoscope_command: Option<&str>,
) -> Result<DiffoscopeOutput, DiffoscopeError> {
    let mut ret = DiffoscopeOutput {
        diffoscope_json_version: Some(1),
        source1: "old version".to_string(),
        source2: "new version".to_string(),
        comments: vec![],
        unified_diff: None,
        details: vec![],
    };

    for ((old_name, old_path), (new_name, new_path)) in old_binaries.iter().zip(new_binaries.iter())
    {
        let sub = _run_diffoscope(old_path, new_path, diffoscope_command, timeout).await?;
        if let Some(mut sub) = sub {
            sub.source1 = old_name.to_string();
            sub.source2 = new_name.to_string();
            sub.diffoscope_json_version = None;
            ret.details.push(sub);
        }
    }
    Ok(ret)
}
