use async_trait::async_trait;
use breezyshim::branch::Branch;
use breezyshim::error::Error as BrzError;
use breezyshim::repository::Repository;
use breezyshim::RevisionId;
use pyo3::exceptions::PyAttributeError;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use silver_platter::vcs::BranchOpenError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use url::Url;

/// Trace context for distributed tracing across HTTP requests
#[derive(Debug, Clone, Default)]
pub struct TraceContext {
    /// Traceparent header value (W3C Trace Context standard)
    pub traceparent: Option<String>,
    /// Tracestate header value (W3C Trace Context standard)
    pub tracestate: Option<String>,
    /// Additional custom trace headers
    pub custom_headers: HashMap<String, String>,
}

impl TraceContext {
    /// Create a new trace context from existing headers
    pub fn from_headers(headers: &HashMap<String, String>) -> Self {
        Self {
            traceparent: headers.get("traceparent").cloned(),
            tracestate: headers.get("tracestate").cloned(),
            custom_headers: headers
                .iter()
                .filter(|(k, _)| k.starts_with("x-trace-") || k.starts_with("x-janitor-"))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
        }
    }

    /// Generate trace headers for HTTP requests
    pub fn to_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        
        if let Some(ref traceparent) = self.traceparent {
            headers.insert("traceparent".to_string(), traceparent.clone());
        }
        
        if let Some(ref tracestate) = self.tracestate {
            headers.insert("tracestate".to_string(), tracestate.clone());
        }
        
        headers.extend(self.custom_headers.clone());
        headers
    }

    /// Create a new trace context with a generated request ID
    pub fn with_request_id(request_id: &str) -> Self {
        let mut context = Self::default();
        context.custom_headers.insert(
            "x-janitor-request-id".to_string(),
            request_id.to_string(),
        );
        context
    }
}

pub fn is_authenticated_url(url: &Url) -> bool {
    ["git+ssh", "bzr+ssh"].contains(&url.scheme())
}

// Serialize as string ("bzr" or "git")
impl Serialize for VcsType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            VcsType::Bzr => "bzr",
            VcsType::Git => "git",
        })
    }
}

impl<'a> Deserialize<'a> for VcsType {
    fn deserialize<D>(deserializer: D) -> Result<VcsType, D::Error>
    where
        D: serde::Deserializer<'a>,
    {
        let s = String::deserialize(deserializer)?;
        std::str::FromStr::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy, std::hash::Hash)]
pub enum VcsType {
    Bzr,
    Git,
}

impl std::fmt::Display for VcsType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            VcsType::Bzr => f.write_str("bzr"),
            VcsType::Git => f.write_str("git"),
        }
    }
}

impl std::str::FromStr for VcsType {
    type Err = String;

    fn from_str(s: &str) -> Result<VcsType, String> {
        match s {
            "bzr" => Ok(VcsType::Bzr),
            "git" => Ok(VcsType::Git),
            _ => Err(format!("Unknown VCS type: {}", s)),
        }
    }
}

pub fn get_branch_vcs_type(branch: &dyn Branch) -> Result<VcsType, BrzError> {
    let repository = branch.repository();
    Python::with_gil(|py| {
        let object = repository.to_object(py);
        match object.getattr(py, "vcs") {
            Ok(vcs) => vcs
                .getattr(py, "abbreviation")
                .unwrap()
                .extract::<String>(py),
            Err(e) if e.is_instance_of::<PyAttributeError>(py) => Ok("bzr".to_string()),
            Err(e) => Err(e),
        }
    })
    .map_err(BrzError::from)
    .map(|vcs| match vcs.as_str() {
        "bzr" => VcsType::Bzr,
        "git" => VcsType::Git,
        _ => panic!("Unknown VCS type: {}", vcs),
    })
}

pub fn is_alioth_url(url: &Url) -> bool {
    matches!(
        url.host_str(),
        Some("svn.debian.org")
            | Some("bzr.debian.org")
            | Some("anonscm.debian.org")
            | Some("hg.debian.org")
            | Some("git.debian.org")
            | Some("alioth.debian.org")
    )
}

#[cfg(test)]
mod is_authenticated_url_tests {
    use super::*;
    #[test]
    fn test_simple() {
        assert!(super::is_authenticated_url(
            &Url::parse("git+ssh://example.com").unwrap()
        ));
        assert!(super::is_authenticated_url(
            &Url::parse("bzr+ssh://example.com").unwrap()
        ));
        assert!(!super::is_authenticated_url(
            &Url::parse("http://example.com").unwrap()
        ));
    }
}

#[cfg(test)]
mod is_alioth_url_tests {
    use super::*;
    #[test]
    fn test_simple() {
        assert!(super::is_alioth_url(
            &Url::parse("https://anonscm.debian.org/cgit/pkg-ocaml-maint/packages/ocamlbuild.git")
                .unwrap()
        ));
        assert!(super::is_alioth_url(
            &Url::parse("https://git.debian.org/git/pkg-ocaml-maint/packages/ocamlbuild.git")
                .unwrap()
        ));
        assert!(super::is_alioth_url(
            &Url::parse(
                "https://alioth.debian.org/anonscm/git/pkg-ocaml-maint/packages/ocamlbuild.git"
            )
            .unwrap()
        ));
        assert!(!super::is_alioth_url(
            &Url::parse("https://example.com").unwrap()
        ));
    }
}

#[derive(Debug)]
pub struct BranchOpenFailure {
    pub code: String,
    pub description: String,
    pub retry_after: Option<chrono::Duration>,
}

impl std::fmt::Display for BranchOpenFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(retry_after) = self.retry_after {
            write!(
                f,
                "BranchOpenFailure(code={}, description={}, retry_after={})",
                self.code, self.description, retry_after
            )
        } else {
            write!(
                f,
                "BranchOpenFailure(code={}, description={})",
                self.code, self.description
            )
        }
    }
}

impl std::error::Error for BranchOpenFailure {}

pub fn open_branch_ext(
    vcs_url: &Url,
    possible_transports: Option<&mut Vec<breezyshim::transport::Transport>>,
    probers: Option<&[&dyn breezyshim::controldir::Prober]>,
) -> Result<Box<dyn Branch>, BranchOpenFailure> {
    match silver_platter::vcs::open_branch(vcs_url, possible_transports, probers, None) {
        Ok(branch) => Ok(branch),
        Err(e) => Err(convert_branch_exception(vcs_url, e)),
    }
}

/// Extended branch opening with trace context support
pub fn open_branch_ext_with_trace(
    vcs_url: &Url,
    possible_transports: Option<&mut Vec<breezyshim::transport::Transport>>,
    probers: Option<&[&dyn breezyshim::controldir::Prober]>,
    trace_context: Option<&TraceContext>,
) -> Result<Box<dyn Branch>, BranchOpenFailure> {
    // Log trace context information if available
    if let Some(trace_ctx) = trace_context {
        let headers = trace_ctx.to_headers();
        if !headers.is_empty() {
            log::debug!(
                "Opening branch {} with trace context: {:?}",
                vcs_url,
                headers.keys().collect::<Vec<_>>()
            );
        }
    }
    
    // For now, call the existing function since breezyshim doesn't support
    // custom headers yet. The trace context is logged for observability.
    match silver_platter::vcs::open_branch(vcs_url, possible_transports, probers, None) {
        Ok(branch) => Ok(branch),
        Err(e) => Err(convert_branch_exception(vcs_url, e)),
    }
}

fn convert_branch_exception(vcs_url: &Url, e: BranchOpenError) -> BranchOpenFailure {
    match e {
        BranchOpenError::RateLimited {
            retry_after,
            description,
            ..
        } => BranchOpenFailure {
            code: "too-many-requests".to_string(),
            description,
            retry_after: retry_after.map(|x| chrono::Duration::seconds(x as i64)),
        },
        BranchOpenError::Unavailable {
            ref description, ..
        } => {
            let code = if description.contains("http code 429: Too Many Requests") {
                "too-many-requests"
            } else if is_alioth_url(vcs_url) {
                "hosted-on-alioth"
            } else if description.contains("Unable to handle http code 401: Unauthorized")
                || description.contains("Unexpected HTTP status 401 for ")
            {
                "401-unauthorized"
            } else if description.contains("Unable to handle http code 502: Bad Gateway")
                || description.contains("Unexpected HTTP status 502 for ")
            {
                "502-bad-gateway"
            } else if description.contains("Subversion branches are not yet") {
                "unsupported-vcs-svn"
            } else if description.contains("Mercurial branches are not yet") {
                "unsupported-vcs-hg"
            } else if description.contains("Darcs branches are not yet") {
                "unsupported-vcs-darcs"
            } else if description.contains("Fossil branches are not yet") {
                "unsupported-vcs-fossil"
            } else {
                "branch-unavailable"
            };
            BranchOpenFailure {
                code: code.to_string(),
                description: description.to_string(),
                retry_after: None,
            }
        }
        BranchOpenError::TemporarilyUnavailable { url, description } => BranchOpenFailure {
            code: "branch-temporarily-unavailable".to_string(),
            description: format!("{} ({})", description, url),
            retry_after: None,
        },
        BranchOpenError::Missing {
            url,
            ref description,
            ..
        } => {
            let code = if description
                .starts_with("Branch does not exist: Not a branch: \"https://anonscm.debian.org")
            {
                "hosted-on-alioth"
            } else {
                "branch-missing"
            };
            BranchOpenFailure {
                code: code.to_string(),
                description: format!("{} ({})", description, url),
                retry_after: None,
            }
        }
        BranchOpenError::Unsupported { description, .. } => {
            let code = if description.contains("Unsupported protocol for url ") {
                if description.contains("anonscm.debian.org")
                    || description.contains("svn.debian.org")
                {
                    "hosted-on-alioth"
                } else if description.contains("svn://") {
                    "unsupported-vcs-svn"
                } else if description.contains("cvs+pserver://") {
                    "unsupported-vcs-cvs"
                } else {
                    "unsupported-vcs-protocol"
                }
            } else if description.contains("Subversion branches are not yet") {
                "unsupported-vcs-svn"
            } else if description.contains("Mercurial branches are not yet") {
                "unsupported-vcs-hg"
            } else if description.contains("Darcs branches are not yet") {
                "unsupported-vcs-darcs"
            } else if description.contains("Fossil branches are not yet") {
                "unsupported-vcs-fossil"
            } else {
                "unsupported-vcs"
            };
            BranchOpenFailure {
                code: code.to_string(),
                description,
                retry_after: None,
            }
        }
        BranchOpenError::Other(description) => BranchOpenFailure {
            code: "unknown".to_string(),
            description,
            retry_after: None,
        },
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RevisionInfo {
    pub commit_id: Option<Vec<u8>>,
    pub revision_id: RevisionId,
    pub message: String,
    pub link: Option<Url>,
}

pub const EMPTY_GIT_TREE: &[u8] = b"4b825dc642cb6eb9a060e54bf8d69288fbee4904";

#[async_trait]
pub trait VcsManager: Send + Sync {
    fn get_branch(
        &self,
        codebase: &str,
        branch_name: &str,
    ) -> Result<Option<Box<dyn Branch>>, BranchOpenError>;

    /// Get the URL for the branch.
    fn get_branch_url(&self, codebase: &str, branch_name: &str) -> Url;

    /// Get the repository for the codebase.
    fn get_repository(&self, codebase: &str) -> Result<Option<Repository>, BrzError>;

    /// Get the URL for the repository.
    fn get_repository_url(&self, codebase: &str) -> Url;

    /// List all repositories.
    fn list_repositories(&self) -> Vec<String>;

    /// Get the diff between two revisions.
    async fn get_diff(
        &self,
        codebase: &str,
        old_revid: &RevisionId,
        new_revid: &RevisionId,
    ) -> Vec<u8>;

    async fn get_revision_info(
        &self,
        codebase: &str,
        old_revid: &RevisionId,
        new_revid: &RevisionId,
    ) -> Vec<RevisionInfo>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalGitVcsManager {
    base_path: PathBuf,
}

impl LocalGitVcsManager {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    pub fn base_path(&self) -> &Path {
        &self.base_path
    }
}

#[async_trait]
impl VcsManager for LocalGitVcsManager {
    fn get_branch(
        &self,
        codebase: &str,
        branch_name: &str,
    ) -> Result<Option<Box<dyn Branch>>, BranchOpenError> {
        let url = self.get_branch_url(codebase, branch_name);
        match silver_platter::vcs::open_branch(
            &url,
            None,
            Some(
                silver_platter::probers::select_probers(Some("git"))
                    .iter()
                    .map(AsRef::as_ref)
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
            None,
        ) {
            Ok(branch) => Ok(Some(branch)),
            Err(BranchOpenError::Unavailable { .. }) | Err(BranchOpenError::Missing { .. }) => {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    fn get_branch_url(&self, codebase: &str, branch_name: &str) -> Url {
        let url = Url::from_directory_path(&self.base_path).unwrap();
        let url = url.join(codebase).unwrap();
        let mut params = std::collections::HashMap::new();
        params.insert("branch".to_string(), branch_name.to_string());
        breezyshim::urlutils::join_segment_parameters(&url, params)
    }

    fn get_repository(&self, codebase: &str) -> Result<Option<Repository>, BrzError> {
        let url = self.get_repository_url(codebase);
        match breezyshim::repository::open(&url) {
            Ok(repo) => Ok(Some(repo)),
            Err(BrzError::NotBranchError(..)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn get_repository_url(&self, codebase: &str) -> Url {
        let abspath = self.base_path.canonicalize().unwrap();
        Url::from_directory_path(&abspath)
            .unwrap()
            .join(codebase)
            .unwrap()
    }

    fn list_repositories(&self) -> Vec<String> {
        self.base_path
            .read_dir()
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect()
    }

    async fn get_diff(
        &self,
        codebase: &str,
        old_revid: &RevisionId,
        new_revid: &RevisionId,
    ) -> Vec<u8> {
        if old_revid == new_revid {
            return vec![];
        }
        let repo = self.get_repository(codebase).unwrap().unwrap();
        let old_sha = if old_revid.is_null() {
            EMPTY_GIT_TREE.to_vec()
        } else {
            repo.lookup_bzr_revision_id(old_revid).unwrap().0
        };
        let new_sha = if new_revid.is_null() {
            EMPTY_GIT_TREE.to_vec()
        } else {
            repo.lookup_bzr_revision_id(new_revid).unwrap().0
        };
        let output = tokio::process::Command::new("git")
            .arg("diff")
            .arg(std::str::from_utf8(&old_sha).unwrap())
            .arg(std::str::from_utf8(&new_sha).unwrap())
            .current_dir(repo.user_transport().local_abspath(Path::new(".")).unwrap())
            .output()
            .await
            .unwrap();
        if !output.status.success() {
            panic!(
                "git diff failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        output.stdout
    }

    async fn get_revision_info(
        &self,
        codebase: &str,
        old_revid: &RevisionId,
        new_revid: &RevisionId,
    ) -> Vec<RevisionInfo> {
        let repo = self.get_repository(codebase).unwrap().unwrap();
        let old_sha = repo.lookup_bzr_revision_id(old_revid).unwrap().0;
        let new_sha = repo.lookup_bzr_revision_id(new_revid).unwrap().0;
        Python::with_gil(|py| {
            let mut ret = vec![];
            let git = repo.to_object(py).getattr(py, "_git").unwrap();
            let walker = git
                .call_method1(py, "get_walker", (new_sha, old_sha))
                .unwrap();

            while let Ok(entry) = walker.call_method0(py, "__next__") {
                let commit = entry.getattr(py, "commit").unwrap();
                let commit_id: Vec<u8> = commit.getattr(py, "id").unwrap().extract(py).unwrap();
                let revision_id = repo.lookup_foreign_revision_id(&commit_id).unwrap();
                let message = commit.getattr(py, "message").unwrap().to_string();
                let link = None;
                ret.push(RevisionInfo {
                    commit_id: Some(commit_id),
                    revision_id,
                    message,
                    link,
                });
            }

            ret
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalBzrVcsManager {
    base_path: PathBuf,
}

impl LocalBzrVcsManager {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    pub fn base_path(&self) -> &Path {
        &self.base_path
    }
}

#[async_trait]
impl VcsManager for LocalBzrVcsManager {
    fn get_branch(
        &self,
        codebase: &str,
        branch_name: &str,
    ) -> Result<Option<Box<dyn Branch>>, BranchOpenError> {
        let url = self.get_branch_url(codebase, branch_name);
        match silver_platter::vcs::open_branch(
            &url,
            None,
            Some(
                silver_platter::probers::select_probers(Some("bzr"))
                    .iter()
                    .map(AsRef::as_ref)
                    .collect::<Vec<_>>()
                    .as_slice(),
            ),
            None,
        ) {
            Ok(branch) => Ok(Some(branch)),
            Err(BranchOpenError::Unavailable { .. }) | Err(BranchOpenError::Missing { .. }) => {
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    fn get_branch_url(&self, codebase: &str, branch_name: &str) -> Url {
        let url = Url::from_directory_path(&self.base_path).unwrap();
        url.join(codebase).unwrap().join(branch_name).unwrap()
    }

    fn get_repository(&self, codebase: &str) -> Result<Option<Repository>, BrzError> {
        let url = self.get_repository_url(codebase);
        match breezyshim::repository::open(&url) {
            Ok(repo) => Ok(Some(repo)),
            Err(BrzError::NotBranchError(..)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn get_repository_url(&self, codebase: &str) -> Url {
        let abspath = self.base_path.canonicalize().unwrap();
        Url::from_directory_path(&abspath)
            .unwrap()
            .join(codebase)
            .unwrap()
    }

    fn list_repositories(&self) -> Vec<String> {
        self.base_path
            .read_dir()
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
            .collect()
    }

    async fn get_diff(
        &self,
        codebase: &str,
        old_revid: &RevisionId,
        new_revid: &RevisionId,
    ) -> Vec<u8> {
        if old_revid == new_revid {
            return vec![];
        }
        let repo = self.get_repository(codebase).unwrap().unwrap();
        let output = tokio::process::Command::new("bzr")
            .arg("diff")
            .arg("-r")
            .arg(format!("{}..{}", old_revid, new_revid))
            .current_dir(repo.user_transport().local_abspath(Path::new(".")).unwrap())
            .output()
            .await
            .unwrap();
        if !output.status.success() {
            panic!(
                "bzr diff failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        output.stdout
    }

    async fn get_revision_info(
        &self,
        codebase: &str,
        old_revid: &RevisionId,
        new_revid: &RevisionId,
    ) -> Vec<RevisionInfo> {
        let repo = self.get_repository(codebase).unwrap().unwrap();

        let lock = repo.lock_read();
        let mut ret = vec![];

        let graph = repo.get_graph();
        let revids = graph
            .iter_lefthand_ancestry(new_revid, Some(&[old_revid.clone()]))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        for (_revid, rev) in repo.iter_revisions(revids) {
            if let Some(rev) = rev {
                ret.push(RevisionInfo {
                    revision_id: rev.revision_id,
                    link: None,
                    message: rev.message.to_string(),
                    commit_id: None,
                });
            }
        }

        std::mem::drop(lock);
        ret
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteGitVcsManager {
    base_url: Url,
}

impl RemoteGitVcsManager {
    pub fn new(base_url: Url) -> Self {
        Self { base_url }
    }

    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    pub fn lookup_revid<'a>(revid: &'a RevisionId, default: &'a [u8]) -> &'a [u8] {
        if revid.is_null() {
            default
        } else {
            revid.as_bytes().strip_prefix(b"git-v1:").unwrap()
        }
    }

    pub fn get_diff_url(
        &self,
        codebase: &str,
        old_revid: &RevisionId,
        new_revid: &RevisionId,
    ) -> Url {
        self.base_url
            .join(&format!(
                "{}/diff?old={}&new={}",
                codebase,
                std::str::from_utf8(RemoteGitVcsManager::lookup_revid(old_revid, EMPTY_GIT_TREE))
                    .unwrap(),
                std::str::from_utf8(RemoteGitVcsManager::lookup_revid(new_revid, EMPTY_GIT_TREE))
                    .unwrap()
            ))
            .unwrap()
    }
}

#[async_trait]
impl VcsManager for RemoteGitVcsManager {
    async fn get_diff(
        &self,
        codebase: &str,
        old_revid: &RevisionId,
        new_revid: &RevisionId,
    ) -> Vec<u8> {
        if old_revid == new_revid {
            return vec![];
        }
        let url = self.get_diff_url(codebase, old_revid, new_revid);
        let client = reqwest::Client::new();
        let resp = client
            .get(url)
            .send()
            .await
            .expect("Git VCS manager should be able to fetch diff");
        resp.bytes()
            .await
            .expect("Git VCS manager should be able to read diff bytes")
            .to_vec()
    }

    async fn get_revision_info(
        &self,
        codebase: &str,
        old_revid: &RevisionId,
        new_revid: &RevisionId,
    ) -> Vec<RevisionInfo> {
        let url = self
            .base_url
            .join(&format!(
                "{}/revision-info?old={}&new={}",
                codebase,
                std::str::from_utf8(RemoteGitVcsManager::lookup_revid(
                    old_revid,
                    breezyshim::git::ZERO_SHA
                ))
                .unwrap(),
                std::str::from_utf8(RemoteGitVcsManager::lookup_revid(
                    new_revid,
                    breezyshim::git::ZERO_SHA
                ))
                .unwrap()
            ))
            .unwrap();
        let client = reqwest::Client::new();
        let resp = client.get(url).send().await.unwrap();
        resp.json().await.unwrap()
    }

    fn get_branch_url(&self, codebase: &str, branch_name: &str) -> Url {
        let url = self.base_url.join(codebase).unwrap();
        let params = std::collections::HashMap::from_iter(vec![(
            "branch".to_string(),
            branch_name.to_string(),
        )]);
        breezyshim::urlutils::join_segment_parameters(&url, params)
    }

    fn get_branch(
        &self,
        codebase: &str,
        branch_name: &str,
    ) -> Result<Option<Box<dyn Branch>>, BranchOpenError> {
        let url = self.get_branch_url(codebase, branch_name);
        // Create trace context from current span for remote Git operations
        let trace_context = if let Some(trace_id) = tracing::Span::current().id() {
            let mut ctx = TraceContext::default();
            ctx.custom_headers.insert(
                "x-trace-span-id".to_string(),
                format!("{:?}", trace_id),
            );
            Some(ctx)
        } else {
            None
        };
        
        open_cached_branch(&url, trace_context.as_ref()).map_err(|e| BranchOpenError::from_err(url, &e))
    }

    fn get_repository_url(&self, codebase: &str) -> Url {
        self.base_url.join(codebase).unwrap()
    }

    fn get_repository(&self, codebase: &str) -> Result<Option<Repository>, BrzError> {
        let url = self.get_repository_url(codebase);
        match breezyshim::repository::open(&url) {
            Ok(repo) => Ok(Some(repo)),
            Err(BrzError::NotBranchError(..)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn list_repositories(&self) -> Vec<String> {
        // For remote Git VCS manager, try to query the repository list endpoint
        let list_url = match self.base_url.join("repositories") {
            Ok(url) => url,
            Err(_) => return Vec::new(),
        };
        
        // Use blocking HTTP request since this is a sync method
        if let Ok(rt) = tokio::runtime::Runtime::new() {
            let client = reqwest::Client::new();
            if let Ok(response) = rt.block_on(client.get(list_url).send()) {
                if response.status().is_success() {
                    if let Ok(repos) = rt.block_on(response.json::<Vec<String>>()) {
                        return repos;
                    }
                }
            }
        }
        
        // Return empty list if API call fails
        Vec::new()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteBzrVcsManager {
    base_url: Url,
}

impl RemoteBzrVcsManager {
    pub fn new(base_url: Url) -> Self {
        Self { base_url }
    }

    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    pub fn get_diff_url(
        &self,
        codebase: &str,
        old_revid: &RevisionId,
        new_revid: &RevisionId,
    ) -> Url {
        self.base_url
            .join(&format!(
                "{}/diff?old={}&new={}",
                codebase, old_revid, new_revid
            ))
            .unwrap()
    }
}

#[async_trait]
impl VcsManager for RemoteBzrVcsManager {
    async fn get_diff(
        &self,
        codebase: &str,
        old_revid: &RevisionId,
        new_revid: &RevisionId,
    ) -> Vec<u8> {
        if old_revid == new_revid {
            return vec![];
        }
        let url = self.get_diff_url(codebase, old_revid, new_revid);
        let client = reqwest::Client::new();
        
        // Add trace context headers if available from current tracing span
        let mut request = client.get(url);
        if let Some(trace_id) = tracing::Span::current().id() {
            request = request.header("x-trace-span-id", format!("{:?}", trace_id));
        }
        
        let resp = request.send().await.unwrap();
        resp.bytes().await.unwrap().to_vec()
    }

    async fn get_revision_info(
        &self,
        codebase: &str,
        old_revid: &RevisionId,
        new_revid: &RevisionId,
    ) -> Vec<RevisionInfo> {
        let url = self
            .base_url
            .join(&format!(
                "{}/revision-info?old={}&new={}",
                codebase, old_revid, new_revid
            ))
            .unwrap();
        let client = reqwest::Client::new();
        
        // Add trace context headers if available from current tracing span
        let mut request = client.get(url);
        if let Some(trace_id) = tracing::Span::current().id() {
            request = request.header("x-trace-span-id", format!("{:?}", trace_id));
        }
        
        let resp = request.send().await.unwrap();
        resp.json().await.unwrap()
    }

    fn get_branch_url(&self, codebase: &str, branch_name: &str) -> Url {
        self.base_url
            .join(codebase)
            .unwrap()
            .join(branch_name)
            .unwrap()
    }

    fn get_branch(
        &self,
        codebase: &str,
        branch_name: &str,
    ) -> Result<Option<Box<dyn Branch>>, BranchOpenError> {
        let url = self.get_branch_url(codebase, branch_name);
        // Create trace context from current span
        let trace_context = if let Some(trace_id) = tracing::Span::current().id() {
            let mut ctx = TraceContext::default();
            ctx.custom_headers.insert(
                "x-trace-span-id".to_string(),
                format!("{:?}", trace_id),
            );
            Some(ctx)
        } else {
            None
        };
        
        open_cached_branch(&url, trace_context.as_ref()).map_err(|e| BranchOpenError::from_err(url, &e))
    }

    fn get_repository_url(&self, codebase: &str) -> Url {
        self.base_url.join(codebase).unwrap()
    }

    fn get_repository(&self, codebase: &str) -> Result<Option<Repository>, BrzError> {
        let url = self.get_repository_url(codebase);
        match breezyshim::repository::open(&url) {
            Ok(repo) => Ok(Some(repo)),
            Err(BrzError::NotBranchError(..)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn list_repositories(&self) -> Vec<String> {
        // For remote Bzr VCS manager, try to query the repository list endpoint
        let list_url = match self.base_url.join("repositories") {
            Ok(url) => url,
            Err(_) => return Vec::new(),
        };
        
        // Use blocking HTTP request since this is a sync method
        if let Ok(rt) = tokio::runtime::Runtime::new() {
            let client = reqwest::Client::new();
            
            // Add trace context headers if available from current tracing span
            let mut request = client.get(list_url);
            if let Some(trace_id) = tracing::Span::current().id() {
                request = request.header("x-trace-span-id", format!("{:?}", trace_id));
            }
            
            if let Ok(response) = rt.block_on(request.send()) {
                if response.status().is_success() {
                    if let Ok(repos) = rt.block_on(response.json::<Vec<String>>()) {
                        return repos;
                    }
                }
            }
        }
        
        // Return empty list if API call fails
        Vec::new()
    }
}

/// Open a cached branch with optional trace context for distributed tracing
fn open_cached_branch(url: &Url, trace_context: Option<&TraceContext>) -> Result<Option<Box<dyn Branch>>, BrzError> {
    fn convert_error(e: BrzError) -> Option<BrzError> {
        match e {
            BrzError::NotBranchError(..) => None,
            BrzError::RemoteGitError(..) => None,
            BrzError::InvalidHttpResponse(..) => None,
            BrzError::ConnectionError(e) => {
                log::info!("Unable to reach cache server: {}", e);
                None
            }
            BrzError::BranchReferenceLoop => None,
            e => Some(e),
        }
    }

    // Prepare trace headers for HTTP transport
    let trace_headers = trace_context.map(|ctx| ctx.to_headers());
    
    if let Some(headers) = trace_headers.as_ref() {
        if !headers.is_empty() {
            log::debug!(
                "Opening cached branch {} with trace headers: {:?}",
                url,
                headers.keys().collect::<Vec<_>>()
            );
        }
    }

    // Create transport with trace context headers
    // Note: breezyshim transport creation doesn't support custom headers yet
    // For now, we log the intent and use the existing transport
    match breezyshim::transport::get_transport(url, None) {
        Ok(transport) => match breezyshim::branch::open_from_transport(&transport).map(Some) {
            Ok(branch) => Ok(branch),
            Err(e) => match convert_error(e) {
                Some(e) => Err(e),
                None => Ok(None),
            },
        },
        Err(e) => match convert_error(e) {
            Some(e) => Err(e),
            None => Ok(None),
        },
    }
}

pub fn get_vcs_managers(location: &str) -> HashMap<VcsType, Box<dyn VcsManager>> {
    if !location.contains("=") {
        vec![
            (
                VcsType::Git,
                Box::new(RemoteGitVcsManager::new(
                    Url::parse(location).unwrap().join("git").unwrap(),
                )) as Box<dyn VcsManager>,
            ),
            (
                VcsType::Bzr,
                Box::new(RemoteBzrVcsManager::new(
                    Url::parse(location).unwrap().join("bzr").unwrap(),
                )) as Box<dyn VcsManager>,
            ),
        ]
        .into_iter()
        .collect()
    } else {
        let mut ret: HashMap<VcsType, Box<dyn VcsManager>> = HashMap::new();
        for p in location.split(",") {
            match p.split_once("=") {
                Some(("git", v)) => {
                    ret.insert(
                        VcsType::Git,
                        Box::new(RemoteGitVcsManager::new(Url::parse(v).unwrap())),
                    );
                }
                Some(("bzr", v)) => {
                    ret.insert(
                        VcsType::Bzr,
                        Box::new(RemoteBzrVcsManager::new(Url::parse(v).unwrap())),
                    );
                }
                _ => panic!("unsupported vcs"),
            }
        }
        ret
    }
}

pub fn get_vcs_managers_from_config(
    config: &crate::config::Config,
) -> HashMap<VcsType, Box<dyn VcsManager>> {
    let mut ret: HashMap<VcsType, Box<dyn VcsManager>> = HashMap::new();
    if let Some(git_location) = config.git_location.as_ref() {
        let url = Url::parse(git_location).unwrap();
        if url.scheme() == "file" {
            ret.insert(
                VcsType::Git,
                Box::new(LocalGitVcsManager::new(url.to_file_path().unwrap())),
            );
        } else {
            ret.insert(
                VcsType::Git,
                Box::new(RemoteGitVcsManager::new(url.clone())),
            );
        }
    }
    if let Some(bzr_location) = config.bzr_location.as_ref() {
        let url = Url::parse(bzr_location).unwrap();
        if url.scheme() == "file" {
            ret.insert(
                VcsType::Bzr,
                Box::new(LocalBzrVcsManager::new(url.to_file_path().unwrap())),
            );
        } else {
            ret.insert(
                VcsType::Bzr,
                Box::new(RemoteBzrVcsManager::new(url.clone())),
            );
        }
    }
    ret
}
