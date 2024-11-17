use patchkit::unified::{iter_hunks, Hunk, HunkLine};
use std::path::PathBuf;
use tracing::{debug, warn};

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[allow(unused)]
pub struct DiffoscopeOutput {
    #[serde(
        rename = "diffoscope-json-version",
        skip_serializing_if = "Option::is_none"
    )]
    diffoscope_json_version: Option<u8>,
    pub source1: PathBuf,
    pub source2: PathBuf,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub comments: Vec<String>,
    #[serde(default)]
    pub unified_diff: Option<String>,
    pub details: Vec<DiffoscopeOutput>,
}

impl pyo3::ToPyObject for DiffoscopeOutput {
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

fn _set_limits(limit_mb: Option<u64>) {
    let limit_mb = limit_mb.unwrap_or(1024);

    nix::sys::resource::setrlimit(
        nix::sys::resource::Resource::RLIMIT_AS,
        limit_mb * 1024 * 1024,
        limit_mb * 1024 * 1024,
    )
    .unwrap();
}

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
    debug!("running {:?}", args);

    let mut cmd = tokio::process::Command::new(&args[0]);
    cmd.args(&args[1..]);
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    if let Some(memory_limit) = memory_limit {
        unsafe {
            cmd.pre_exec(move || {
                _set_limits(Some(memory_limit as u64));
                Ok(())
            })
        };
    }

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

pub fn format_diffoscope(
    diff: &DiffoscopeOutput,
    content_type: &str,
    title: &str,
    css_url: Option<&str>,
) -> String {
    use pyo3::prelude::*;
    if content_type == "application/json" {
        return serde_json::to_string(diff).unwrap();
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
                sysm.setattr("stdout", p.getattr("stdout")?)?;
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
                let out = pyo3::types::PyList::empty_bound(py);

                let presenter = m
                    .getattr("MarkdownPresenter")?
                    .call1((out.getattr("append")?,))?;
                let markdown = presenter.call_method1("start", (root_differ,))?;
                Ok(markdown.extract::<String>()?)
            }
            "text/plain" => {
                let m = py.import_bound("diffoscope.presenters.text")?;
                let out = pyo3::types::PyString::new_bound(py, "");

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
    .unwrap()
}

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
