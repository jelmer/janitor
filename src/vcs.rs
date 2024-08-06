use breezyshim::branch::Branch;
use breezyshim::error::Error as BrzError;
use pyo3::exceptions::PyAttributeError;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
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

fn convert_branch_exception(
    vcs_url: &Url,
    e: silver_platter::vcs::BranchOpenError,
) -> BranchOpenFailure {
    match e {
        silver_platter::vcs::BranchOpenError::RateLimited {
            retry_after,
            description,
            ..
        } => BranchOpenFailure {
            code: "too-many-requests".to_string(),
            description,
            retry_after: retry_after.map(|x| chrono::Duration::seconds(x as i64)),
        },
        silver_platter::vcs::BranchOpenError::Unavailable {
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
        silver_platter::vcs::BranchOpenError::TemporarilyUnavailable { url, description } => {
            BranchOpenFailure {
                code: "branch-temporarily-unavailable".to_string(),
                description: format!("{} ({})", description, url),
                retry_after: None,
            }
        }
        silver_platter::vcs::BranchOpenError::Missing {
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
        silver_platter::vcs::BranchOpenError::Unsupported { description, .. } => {
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
        silver_platter::vcs::BranchOpenError::Other(description) => BranchOpenFailure {
            code: "unknown".to_string(),
            description,
            retry_after: None,
        },
    }
}
