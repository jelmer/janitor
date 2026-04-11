use async_trait::async_trait;
use breezyshim::branch::Branch;
use breezyshim::error::Error as BrzError;
use breezyshim::repository::{GenericRepository, PyRepository, Repository};
use breezyshim::RevisionId;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use silver_platter::vcs::BranchOpenError;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use url::Url;

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
    match repository.vcs_type() {
        breezyshim::foreign::VcsType::Git => Ok(VcsType::Git),
        breezyshim::foreign::VcsType::Bazaar => Ok(VcsType::Bzr),
        other => panic!("Unknown VCS type: {:?}", other),
    }
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
mod tests {
    use super::*;

    #[test]
    fn test_authenticated_urls() {
        assert!(is_authenticated_url(
            &Url::parse("git+ssh://example.com").unwrap()
        ));
        assert!(is_authenticated_url(
            &Url::parse("bzr+ssh://example.com").unwrap()
        ));
        assert!(is_authenticated_url(
            &Url::parse("git+ssh://git.example.com/repo.git").unwrap()
        ));
    }

    #[test]
    fn test_not_authenticated_urls() {
        assert!(!is_authenticated_url(
            &Url::parse("http://example.com").unwrap()
        ));
        assert!(!is_authenticated_url(
            &Url::parse("https://example.com").unwrap()
        ));
        assert!(!is_authenticated_url(
            &Url::parse("git://example.com/repo.git").unwrap()
        ));
    }

    #[test]
    fn test_alioth_urls() {
        assert!(is_alioth_url(
            &Url::parse("https://anonscm.debian.org/cgit/pkg-ocaml-maint/packages/ocamlbuild.git")
                .unwrap()
        ));
        assert!(is_alioth_url(
            &Url::parse("https://git.debian.org/git/pkg-ocaml-maint/packages/ocamlbuild.git")
                .unwrap()
        ));
        assert!(is_alioth_url(
            &Url::parse(
                "https://alioth.debian.org/anonscm/git/pkg-ocaml-maint/packages/ocamlbuild.git"
            )
            .unwrap()
        ));
        assert!(is_alioth_url(
            &Url::parse("https://svn.debian.org/svn/some-pkg").unwrap()
        ));
        assert!(is_alioth_url(
            &Url::parse("https://bzr.debian.org/bzr/some-pkg").unwrap()
        ));
        assert!(is_alioth_url(
            &Url::parse("https://hg.debian.org/hg/some-pkg").unwrap()
        ));
    }

    #[test]
    fn test_not_alioth_urls() {
        assert!(!is_alioth_url(&Url::parse("https://example.com").unwrap()));
        assert!(!is_alioth_url(
            &Url::parse("https://salsa.debian.org/foo/bar").unwrap()
        ));
        assert!(!is_alioth_url(
            &Url::parse("https://github.com/foo/bar").unwrap()
        ));
    }

    #[test]
    fn test_vcs_type_from_str() {
        assert_eq!("git".parse::<VcsType>().unwrap(), VcsType::Git);
        assert_eq!("bzr".parse::<VcsType>().unwrap(), VcsType::Bzr);
        assert!("svn".parse::<VcsType>().is_err());
        assert!("".parse::<VcsType>().is_err());
    }

    #[test]
    fn test_vcs_type_display() {
        assert_eq!(VcsType::Git.to_string(), "git");
        assert_eq!(VcsType::Bzr.to_string(), "bzr");
    }

    #[test]
    fn test_vcs_type_serde_roundtrip() {
        let git = VcsType::Git;
        let json = serde_json::to_string(&git).unwrap();
        assert_eq!(json, r#""git""#);
        let roundtripped: VcsType = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped, git);

        let bzr = VcsType::Bzr;
        let json = serde_json::to_string(&bzr).unwrap();
        assert_eq!(json, r#""bzr""#);
        let roundtripped: VcsType = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtripped, bzr);
    }

    #[test]
    fn test_vcs_type_serde_invalid() {
        assert!(serde_json::from_str::<VcsType>(r#""svn""#).is_err());
    }

    #[test]
    fn test_convert_branch_exception_rate_limited() {
        let err = BranchOpenError::RateLimited {
            url: Url::parse("https://example.com").unwrap(),
            description: "Too many requests".to_string(),
            retry_after: Some(60.0),
        };
        let failure = convert_branch_exception(&Url::parse("https://example.com").unwrap(), err);
        assert_eq!(failure.code, "too-many-requests");
        assert_eq!(failure.retry_after, Some(chrono::Duration::seconds(60)));
    }

    #[test]
    fn test_convert_branch_exception_missing() {
        let err = BranchOpenError::Missing {
            url: Url::parse("https://example.com/repo").unwrap(),
            description: "Not found".to_string(),
        };
        let failure =
            convert_branch_exception(&Url::parse("https://example.com/repo").unwrap(), err);
        assert_eq!(failure.code, "branch-missing");
    }

    #[test]
    fn test_convert_branch_exception_missing_alioth() {
        let err = BranchOpenError::Missing {
            url: Url::parse("https://anonscm.debian.org/repo").unwrap(),
            description: "Branch does not exist: Not a branch: \"https://anonscm.debian.org/repo\""
                .to_string(),
        };
        let failure =
            convert_branch_exception(&Url::parse("https://anonscm.debian.org/repo").unwrap(), err);
        assert_eq!(failure.code, "hosted-on-alioth");
    }

    #[test]
    fn test_convert_branch_exception_unavailable_alioth() {
        let err = BranchOpenError::Unavailable {
            url: Url::parse("https://anonscm.debian.org/repo").unwrap(),
            description: "Connection refused".to_string(),
        };
        let failure =
            convert_branch_exception(&Url::parse("https://anonscm.debian.org/repo").unwrap(), err);
        assert_eq!(failure.code, "hosted-on-alioth");
    }

    #[test]
    fn test_convert_branch_exception_unavailable_401() {
        let err = BranchOpenError::Unavailable {
            url: Url::parse("https://example.com/repo").unwrap(),
            description: "Unable to handle http code 401: Unauthorized".to_string(),
        };
        let failure =
            convert_branch_exception(&Url::parse("https://example.com/repo").unwrap(), err);
        assert_eq!(failure.code, "401-unauthorized");
    }

    #[test]
    fn test_convert_branch_exception_unsupported_svn() {
        let err = BranchOpenError::Unsupported {
            url: Url::parse("svn://example.com/repo").unwrap(),
            description: "Unsupported protocol for url svn://example.com/repo".to_string(),
            vcs: None,
        };
        let failure = convert_branch_exception(&Url::parse("svn://example.com/repo").unwrap(), err);
        assert_eq!(failure.code, "unsupported-vcs-svn");
    }

    #[test]
    fn test_convert_branch_exception_temporarily_unavailable() {
        let err = BranchOpenError::TemporarilyUnavailable {
            url: Url::parse("https://example.com/repo").unwrap(),
            description: "Server busy".to_string(),
        };
        let failure =
            convert_branch_exception(&Url::parse("https://example.com/repo").unwrap(), err);
        assert_eq!(failure.code, "branch-temporarily-unavailable");
    }

    #[test]
    fn test_convert_branch_exception_other() {
        let err = BranchOpenError::Other("something weird".to_string());
        let failure = convert_branch_exception(&Url::parse("https://example.com").unwrap(), err);
        assert_eq!(failure.code, "unknown");
        assert_eq!(failure.description, "something weird");
    }

    #[test]
    fn test_get_vcs_managers_simple_url() {
        let managers = get_vcs_managers("https://vcs.example.com/");
        assert!(managers.contains_key(&VcsType::Git));
        assert!(managers.contains_key(&VcsType::Bzr));
    }

    #[test]
    fn test_get_vcs_managers_explicit() {
        let managers =
            get_vcs_managers("git=https://git.example.com/,bzr=https://bzr.example.com/");
        assert!(managers.contains_key(&VcsType::Git));
        assert!(managers.contains_key(&VcsType::Bzr));
    }

    #[test]
    fn test_get_vcs_managers_git_only() {
        let managers = get_vcs_managers("git=https://git.example.com/");
        assert!(managers.contains_key(&VcsType::Git));
        assert!(!managers.contains_key(&VcsType::Bzr));
    }

    #[test]
    fn test_branch_open_failure_display() {
        let failure = BranchOpenFailure {
            code: "branch-missing".to_string(),
            description: "Not found".to_string(),
            retry_after: None,
        };
        assert_eq!(
            failure.to_string(),
            "BranchOpenFailure(code=branch-missing, description=Not found)"
        );

        let failure_with_retry = BranchOpenFailure {
            code: "too-many-requests".to_string(),
            description: "Slow down".to_string(),
            retry_after: Some(chrono::Duration::seconds(60)),
        };
        assert_eq!(
            failure_with_retry.to_string(),
            "BranchOpenFailure(code=too-many-requests, description=Slow down, retry_after=PT60S)"
        );
    }

    #[test]
    fn test_remote_git_vcs_manager_diff_url() {
        let mgr = RemoteGitVcsManager::new(Url::parse("https://vcs.example.com/git/").unwrap());
        let old = RevisionId::from(b"git-v1:aaaa".to_vec());
        let new = RevisionId::from(b"git-v1:bbbb".to_vec());
        let url = mgr.get_diff_url("mycodebase", &old, &new);
        assert_eq!(
            url.as_str(),
            "https://vcs.example.com/git/mycodebase/diff?old=aaaa&new=bbbb"
        );
    }

    #[test]
    fn test_remote_bzr_vcs_manager_branch_url() {
        let mgr = RemoteBzrVcsManager::new(Url::parse("https://vcs.example.com/bzr/").unwrap());
        let url = mgr.get_branch_url("mycodebase", "main");
        assert_eq!(url.as_str(), "https://vcs.example.com/bzr/mycodebase/main");
    }

    #[test]
    fn test_remote_bzr_vcs_manager_diff_url() {
        let mgr = RemoteBzrVcsManager::new(Url::parse("https://vcs.example.com/bzr/").unwrap());
        let old = RevisionId::from(b"old-revid".to_vec());
        let new = RevisionId::from(b"new-revid".to_vec());
        let url = mgr.get_diff_url("mycodebase", &old, &new);
        assert_eq!(
            url.as_str(),
            "https://vcs.example.com/bzr/mycodebase/diff?old=old-revid&new=new-revid"
        );
    }

    #[test]
    fn test_remote_git_vcs_manager_repository_url() {
        let mgr = RemoteGitVcsManager::new(Url::parse("https://vcs.example.com/git/").unwrap());
        assert_eq!(
            mgr.get_repository_url("mycodebase").as_str(),
            "https://vcs.example.com/git/mycodebase"
        );
    }

    #[test]
    fn test_remote_bzr_vcs_manager_repository_url() {
        let mgr = RemoteBzrVcsManager::new(Url::parse("https://vcs.example.com/bzr/").unwrap());
        assert_eq!(
            mgr.get_repository_url("mycodebase").as_str(),
            "https://vcs.example.com/bzr/mycodebase"
        );
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
    probers: Option<&[&dyn breezyshim::controldir::PyProber]>,
) -> Result<breezyshim::branch::GenericBranch, BranchOpenFailure> {
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
    ) -> Result<Option<breezyshim::branch::GenericBranch>, BranchOpenError>;

    /// Get the URL for the branch.
    fn get_branch_url(&self, codebase: &str, branch_name: &str) -> Url;

    /// Get the repository for the codebase.
    fn get_repository(&self, codebase: &str) -> Result<Option<GenericRepository>, BrzError>;

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
    ) -> Result<Option<breezyshim::branch::GenericBranch>, BranchOpenError> {
        let url = self.get_branch_url(codebase, branch_name);
        let probers = silver_platter::probers::select_probers(Some("git"));
        match silver_platter::vcs::open_branch(
            &url,
            None,
            Some(
                probers
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

    fn get_repository(&self, codebase: &str) -> Result<Option<GenericRepository>, BrzError> {
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
        let current_dir = repo.user_transport().local_abspath(Path::new(".")).unwrap();
        drop(repo); // Drop repo before await
        let output = tokio::process::Command::new("git")
            .arg("diff")
            .arg(std::str::from_utf8(&old_sha).unwrap())
            .arg(std::str::from_utf8(&new_sha).unwrap())
            .current_dir(current_dir)
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

        // Collect all the info we need from the Python objects before dropping repo
        let commit_infos: Vec<(Vec<u8>, String)> = Python::attach(|py| {
            let mut ret = vec![];
            let repo_obj = repo.to_object(py);
            let git = repo_obj.getattr(py, "_git").unwrap();
            let walker = git
                .call_method1(py, "get_walker", (new_sha, old_sha))
                .unwrap();

            while let Ok(entry) = walker.call_method0(py, "__next__") {
                let commit: Py<PyAny> = entry.getattr(py, "commit").unwrap();
                let commit_id: Vec<u8> = commit.getattr(py, "id").unwrap().extract(py).unwrap();
                let message = commit.getattr(py, "message").unwrap().to_string();
                ret.push((commit_id, message));
            }

            ret
        });

        // Now convert commit_ids to revision_ids
        let mut ret = vec![];
        for (commit_id, message) in commit_infos {
            let revision_id = repo.lookup_foreign_revision_id(&commit_id).unwrap();
            ret.push(RevisionInfo {
                commit_id: Some(commit_id),
                revision_id,
                message,
                link: None,
            });
        }

        ret
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
    ) -> Result<Option<breezyshim::branch::GenericBranch>, BranchOpenError> {
        let url = self.get_branch_url(codebase, branch_name);
        let probers = silver_platter::probers::select_probers(Some("bzr"));
        match silver_platter::vcs::open_branch(
            &url,
            None,
            Some(
                probers
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
        url.join(&format!("{}/", codebase))
            .unwrap()
            .join(branch_name)
            .unwrap()
    }

    fn get_repository(&self, codebase: &str) -> Result<Option<GenericRepository>, BrzError> {
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
            .unwrap()
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
        let resp = client.get(url).send().await.unwrap();
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
    ) -> Result<Option<breezyshim::branch::GenericBranch>, BranchOpenError> {
        let url = self.get_branch_url(codebase, branch_name);
        open_cached_branch(&url).map_err(|e| BranchOpenError::from_err(url, &e))
    }

    fn get_repository_url(&self, codebase: &str) -> Url {
        self.base_url.join(codebase).unwrap()
    }

    fn get_repository(&self, codebase: &str) -> Result<Option<GenericRepository>, BrzError> {
        let url = self.get_repository_url(codebase);
        match breezyshim::repository::open(&url) {
            Ok(repo) => Ok(Some(repo)),
            Err(BrzError::NotBranchError(..)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn list_repositories(&self) -> Vec<String> {
        todo!()
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
        let resp = client.get(url).send().await.unwrap();
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
        let resp = client.get(url).send().await.unwrap();
        resp.json().await.unwrap()
    }

    fn get_branch_url(&self, codebase: &str, branch_name: &str) -> Url {
        self.base_url
            .join(&format!("{}/", codebase))
            .unwrap()
            .join(branch_name)
            .unwrap()
    }

    fn get_branch(
        &self,
        codebase: &str,
        branch_name: &str,
    ) -> Result<Option<breezyshim::branch::GenericBranch>, BranchOpenError> {
        let url = self.get_branch_url(codebase, branch_name);
        open_cached_branch(&url).map_err(|e| BranchOpenError::from_err(url, &e))
    }

    fn get_repository_url(&self, codebase: &str) -> Url {
        self.base_url.join(codebase).unwrap()
    }

    fn get_repository(&self, codebase: &str) -> Result<Option<GenericRepository>, BrzError> {
        let url = self.get_repository_url(codebase);
        match breezyshim::repository::open(&url) {
            Ok(repo) => Ok(Some(repo)),
            Err(BrzError::NotBranchError(..)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    fn list_repositories(&self) -> Vec<String> {
        todo!()
    }
}

fn open_cached_branch(url: &Url) -> Result<Option<breezyshim::branch::GenericBranch>, BrzError> {
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

    // TODO(jelmer): Somehow pass in trace context headers
    match breezyshim::transport::get_transport(url, None) {
        Ok(transport) => match breezyshim::branch::open_from_transport_as_generic(&transport) {
            Ok(branch) => Ok(Some(branch)),
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
