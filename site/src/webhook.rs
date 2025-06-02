use anyhow::{Context, Result};
use axum::{
    body::Bytes,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, error, info, warn};
use url::Url;

use crate::app::AppState;

/// Webhook event types for different VCS platforms
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "platform", content = "event")]
pub enum WebhookEvent {
    /// GitHub webhook events
    GitHub {
        event_type: String,
        repository: GitHubRepository,
        commits: Vec<GitHubCommit>,
        head_commit: Option<GitHubCommit>,
        #[serde(rename = "ref")]
        git_ref: String,
    },
    /// GitLab webhook events
    GitLab {
        event_name: String,
        project: GitLabProject,
        commits: Vec<GitLabCommit>,
        #[serde(rename = "ref")]
        git_ref: String,
    },
    /// Gitea webhook events
    Gitea {
        repository: GiteaRepository,
        commits: Vec<GiteaCommit>,
        #[serde(rename = "ref")]
        git_ref: String,
    },
    /// Gogs webhook events
    Gogs {
        repository: GogsRepository,
        commits: Vec<GogsCommit>,
        #[serde(rename = "ref")]
        git_ref: String,
    },
    /// Launchpad webhook events
    Launchpad {
        bzr_branch: Option<String>,
        git_repository: Option<String>,
        new_revno: Option<i32>,
        new_revid: Option<String>,
    },
}

/// GitHub repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRepository {
    pub name: String,
    pub full_name: String,
    pub html_url: String,
    pub clone_url: String,
    pub ssh_url: String,
    pub git_url: String,
    pub default_branch: String,
}

/// GitHub commit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubCommit {
    pub id: String,
    pub message: String,
    pub url: String,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub modified: Vec<String>,
}

/// GitLab project information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabProject {
    pub name: String,
    pub path_with_namespace: String,
    pub web_url: String,
    pub http_url: String,
    pub ssh_url: String,
    pub default_branch: String,
}

/// GitLab commit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitLabCommit {
    pub id: String,
    pub message: String,
    pub url: String,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub modified: Vec<String>,
}

/// Gitea repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaRepository {
    pub name: String,
    pub full_name: String,
    pub html_url: String,
    pub clone_url: String,
    pub ssh_url: String,
    pub default_branch: String,
}

/// Gitea commit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GiteaCommit {
    pub id: String,
    pub message: String,
    pub url: String,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub modified: Vec<String>,
}

/// Gogs repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GogsRepository {
    pub name: String,
    pub full_name: String,
    pub html_url: String,
    pub clone_url: String,
    pub ssh_url: String,
    pub default_branch: String,
}

/// Gogs commit information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GogsCommit {
    pub id: String,
    pub message: String,
    pub url: String,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub modified: Vec<String>,
}

/// VCS change representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VcsChange {
    Git {
        urls: Vec<String>,
        commit_sha: String,
        branch: String,
    },
    Bzr {
        urls: Vec<String>,
        revno: i32,
        revid: String,
        branch: String,
    },
}

/// Webhook signature verification configuration
#[derive(Debug, Clone)]
pub struct WebhookConfig {
    /// Secret key for HMAC signature verification
    pub secret: Option<String>,
    /// Enable signature verification
    pub verify_signatures: bool,
    /// Allowed webhook origins (for CORS)
    pub allowed_origins: Vec<String>,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            secret: None,
            verify_signatures: false,
            allowed_origins: vec!["*".to_string()],
        }
    }
}

/// Webhook processor for handling incoming VCS events
pub struct WebhookProcessor {
    config: WebhookConfig,
}

impl WebhookProcessor {
    /// Create a new webhook processor
    pub fn new(config: WebhookConfig) -> Self {
        Self { config }
    }

    /// Detect if a request is a webhook from headers
    pub fn is_webhook_request(&self, headers: &HeaderMap) -> bool {
        headers.contains_key("x-github-event")
            || headers.contains_key("x-gitlab-event")
            || headers.contains_key("x-gitea-event")
            || headers.contains_key("x-gogs-event")
            || headers.contains_key("x-launchpad-event-type")
    }

    /// Verify webhook signature using HMAC-SHA256
    pub fn verify_signature(&self, headers: &HeaderMap, payload: &[u8]) -> Result<bool> {
        if !self.config.verify_signatures {
            return Ok(true);
        }

        let Some(secret) = &self.config.secret else {
            warn!("Signature verification enabled but no secret configured");
            return Ok(false);
        };

        let signature_header = headers
            .get("x-janitor-signature")
            .or_else(|| headers.get("x-hub-signature-256"))
            .or_else(|| headers.get("x-gitlab-token"));

        let Some(signature_header) = signature_header else {
            warn!("No signature header found in webhook request");
            return Ok(false);
        };

        let signature = signature_header
            .to_str()
            .context("Invalid signature header encoding")?;

        // For GitHub-style signatures, remove the "sha256=" prefix
        let signature = if signature.starts_with("sha256=") {
            &signature[7..]
        } else {
            signature
        };

        // Calculate expected HMAC-SHA256
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type HmacSha256 = Hmac<Sha256>;

        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).context("Invalid secret key for HMAC")?;
        mac.update(payload);
        let expected = mac.finalize().into_bytes();
        let expected_hex = hex::encode(expected);

        Ok(signature == expected_hex)
    }

    /// Parse webhook payload based on platform headers
    pub fn parse_webhook(&self, headers: &HeaderMap, body: &[u8]) -> Result<Option<WebhookEvent>> {
        // Determine platform from headers
        if let Some(github_event) = headers.get("x-github-event") {
            self.parse_github_webhook(github_event.to_str()?, body)
        } else if let Some(gitlab_event) = headers.get("x-gitlab-event") {
            self.parse_gitlab_webhook(gitlab_event.to_str()?, body)
        } else if headers.contains_key("x-gitea-event") {
            self.parse_gitea_webhook(body)
        } else if headers.contains_key("x-gogs-event") {
            self.parse_gogs_webhook(body)
        } else if let Some(launchpad_event) = headers.get("x-launchpad-event-type") {
            self.parse_launchpad_webhook(launchpad_event.to_str()?, body)
        } else {
            Ok(None)
        }
    }

    /// Parse GitHub webhook payload
    fn parse_github_webhook(&self, event_type: &str, body: &[u8]) -> Result<Option<WebhookEvent>> {
        if event_type != "push" {
            debug!("Ignoring GitHub {} event", event_type);
            return Ok(None);
        }

        let payload: Value =
            serde_json::from_slice(body).context("Failed to parse GitHub webhook JSON")?;

        let repository: GitHubRepository = serde_json::from_value(payload["repository"].clone())
            .context("Failed to parse GitHub repository")?;

        let commits: Vec<GitHubCommit> = serde_json::from_value(payload["commits"].clone())
            .context("Failed to parse GitHub commits")?;

        let head_commit = if payload["head_commit"].is_null() {
            None
        } else {
            Some(
                serde_json::from_value(payload["head_commit"].clone())
                    .context("Failed to parse GitHub head commit")?,
            )
        };

        let git_ref = payload["ref"]
            .as_str()
            .context("Missing ref in GitHub webhook")?
            .to_string();

        Ok(Some(WebhookEvent::GitHub {
            event_type: event_type.to_string(),
            repository,
            commits,
            head_commit,
            git_ref,
        }))
    }

    /// Parse GitLab webhook payload
    fn parse_gitlab_webhook(&self, event_name: &str, body: &[u8]) -> Result<Option<WebhookEvent>> {
        if event_name != "Push Hook" {
            debug!("Ignoring GitLab {} event", event_name);
            return Ok(None);
        }

        let payload: Value =
            serde_json::from_slice(body).context("Failed to parse GitLab webhook JSON")?;

        let project: GitLabProject = serde_json::from_value(payload["project"].clone())
            .context("Failed to parse GitLab project")?;

        let commits: Vec<GitLabCommit> = serde_json::from_value(payload["commits"].clone())
            .context("Failed to parse GitLab commits")?;

        let git_ref = payload["ref"]
            .as_str()
            .context("Missing ref in GitLab webhook")?
            .to_string();

        Ok(Some(WebhookEvent::GitLab {
            event_name: event_name.to_string(),
            project,
            commits,
            git_ref,
        }))
    }

    /// Parse Gitea webhook payload
    fn parse_gitea_webhook(&self, body: &[u8]) -> Result<Option<WebhookEvent>> {
        let payload: Value =
            serde_json::from_slice(body).context("Failed to parse Gitea webhook JSON")?;

        let repository: GiteaRepository = serde_json::from_value(payload["repository"].clone())
            .context("Failed to parse Gitea repository")?;

        let commits: Vec<GiteaCommit> = serde_json::from_value(payload["commits"].clone())
            .context("Failed to parse Gitea commits")?;

        let git_ref = payload["ref"]
            .as_str()
            .context("Missing ref in Gitea webhook")?
            .to_string();

        Ok(Some(WebhookEvent::Gitea {
            repository,
            commits,
            git_ref,
        }))
    }

    /// Parse Gogs webhook payload
    fn parse_gogs_webhook(&self, body: &[u8]) -> Result<Option<WebhookEvent>> {
        let payload: Value =
            serde_json::from_slice(body).context("Failed to parse Gogs webhook JSON")?;

        let repository: GogsRepository = serde_json::from_value(payload["repository"].clone())
            .context("Failed to parse Gogs repository")?;

        let commits: Vec<GogsCommit> = serde_json::from_value(payload["commits"].clone())
            .context("Failed to parse Gogs commits")?;

        let git_ref = payload["ref"]
            .as_str()
            .context("Missing ref in Gogs webhook")?
            .to_string();

        Ok(Some(WebhookEvent::Gogs {
            repository,
            commits,
            git_ref,
        }))
    }

    /// Parse Launchpad webhook payload
    fn parse_launchpad_webhook(
        &self,
        event_type: &str,
        body: &[u8],
    ) -> Result<Option<WebhookEvent>> {
        match event_type {
            "bzr:push:0.1" | "git:push:0.1" => {
                let payload: Value = serde_json::from_slice(body)
                    .context("Failed to parse Launchpad webhook JSON")?;

                Ok(Some(WebhookEvent::Launchpad {
                    bzr_branch: payload["bzr_branch"].as_str().map(|s| s.to_string()),
                    git_repository: payload["git_repository"].as_str().map(|s| s.to_string()),
                    new_revno: payload["new_revno"].as_i64().map(|n| n as i32),
                    new_revid: payload["new_revid"].as_str().map(|s| s.to_string()),
                }))
            }
            _ => {
                debug!("Ignoring Launchpad {} event", event_type);
                Ok(None)
            }
        }
    }

    /// Convert webhook event to VCS change
    pub fn webhook_to_vcs_change(&self, event: &WebhookEvent) -> Result<Option<VcsChange>> {
        match event {
            WebhookEvent::GitHub {
                repository,
                head_commit,
                git_ref,
                ..
            } => {
                let Some(head_commit) = head_commit else {
                    return Ok(None);
                };

                let branch = self.extract_branch_from_ref(git_ref);
                let urls = vec![
                    repository.clone_url.clone(),
                    repository.ssh_url.clone(),
                    repository.git_url.clone(),
                    repository.html_url.clone(),
                ];

                Ok(Some(VcsChange::Git {
                    urls,
                    commit_sha: head_commit.id.clone(),
                    branch,
                }))
            }
            WebhookEvent::GitLab {
                project,
                commits,
                git_ref,
                ..
            } => {
                let Some(latest_commit) = commits.last() else {
                    return Ok(None);
                };

                let branch = self.extract_branch_from_ref(git_ref);
                let urls = vec![
                    project.http_url.clone(),
                    project.ssh_url.clone(),
                    project.web_url.clone(),
                ];

                Ok(Some(VcsChange::Git {
                    urls,
                    commit_sha: latest_commit.id.clone(),
                    branch,
                }))
            }
            WebhookEvent::Gitea {
                repository,
                commits,
                git_ref,
            } => {
                let Some(latest_commit) = commits.last() else {
                    return Ok(None);
                };

                let branch = self.extract_branch_from_ref(git_ref);
                let urls = vec![
                    repository.clone_url.clone(),
                    repository.ssh_url.clone(),
                    repository.html_url.clone(),
                ];

                Ok(Some(VcsChange::Git {
                    urls,
                    commit_sha: latest_commit.id.clone(),
                    branch,
                }))
            }
            WebhookEvent::Gogs {
                repository,
                commits,
                git_ref,
            } => {
                let Some(latest_commit) = commits.last() else {
                    return Ok(None);
                };

                let branch = self.extract_branch_from_ref(git_ref);
                let urls = vec![
                    repository.clone_url.clone(),
                    repository.ssh_url.clone(),
                    repository.html_url.clone(),
                ];

                Ok(Some(VcsChange::Git {
                    urls,
                    commit_sha: latest_commit.id.clone(),
                    branch,
                }))
            }
            WebhookEvent::Launchpad {
                bzr_branch,
                git_repository,
                new_revno,
                new_revid,
            } => {
                if let (Some(bzr_branch), Some(revno), Some(revid)) =
                    (bzr_branch, new_revno, new_revid)
                {
                    Ok(Some(VcsChange::Bzr {
                        urls: vec![bzr_branch.clone()],
                        revno: *revno,
                        revid: revid.clone(),
                        branch: "main".to_string(), // Bzr typically uses main/trunk
                    }))
                } else if let Some(git_repo) = git_repository {
                    // For Launchpad Git repositories, we need more information
                    // This is a simplified implementation
                    Ok(Some(VcsChange::Git {
                        urls: vec![git_repo.clone()],
                        commit_sha: new_revid.clone().unwrap_or_default(),
                        branch: "main".to_string(),
                    }))
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Extract branch name from Git ref
    fn extract_branch_from_ref(&self, git_ref: &str) -> String {
        if git_ref.starts_with("refs/heads/") {
            git_ref
                .strip_prefix("refs/heads/")
                .unwrap_or(git_ref)
                .to_string()
        } else {
            git_ref.to_string()
        }
    }
}

/// Webhook registration client for subscribing to VCS events
pub struct WebhookRegistration {
    http_client: reqwest::Client,
}

impl WebhookRegistration {
    /// Create a new webhook registration client
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
        }
    }

    /// Subscribe to GitHub repository webhooks
    pub async fn subscribe_github_webhook(
        &self,
        repo_url: &str,
        webhook_url: &str,
        token: &str,
    ) -> Result<()> {
        let repo_path = self.extract_github_repo_path(repo_url)?;
        let api_url = format!("https://api.github.com/repos/{}/hooks", repo_path);

        let payload = json!({
            "name": "web",
            "active": true,
            "events": ["push"],
            "config": {
                "url": webhook_url,
                "content_type": "json",
                "insecure_ssl": "0"
            }
        });

        let response = self
            .http_client
            .post(&api_url)
            .header("Authorization", format!("token {}", token))
            .header("User-Agent", "Janitor")
            .json(&payload)
            .send()
            .await
            .context("Failed to send GitHub webhook subscription request")?;

        if response.status().is_success() || response.status() == StatusCode::UNPROCESSABLE_ENTITY {
            info!("GitHub webhook subscription successful for {}", repo_path);
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("GitHub webhook subscription failed: {}", error_text);
        }
    }

    /// Subscribe to GitLab project webhooks
    pub async fn subscribe_gitlab_webhook(
        &self,
        project_url: &str,
        webhook_url: &str,
        token: &str,
    ) -> Result<()> {
        let project_id = self.extract_gitlab_project_id(project_url)?;
        let api_url = format!("https://gitlab.com/api/v4/projects/{}/hooks", project_id);

        let payload = json!({
            "url": webhook_url,
            "push_events": true,
            "enable_ssl_verification": true
        });

        let response = self
            .http_client
            .post(&api_url)
            .header("Private-Token", token)
            .json(&payload)
            .send()
            .await
            .context("Failed to send GitLab webhook subscription request")?;

        if response.status().is_success() || response.status() == StatusCode::UNPROCESSABLE_ENTITY {
            info!(
                "GitLab webhook subscription successful for project {}",
                project_id
            );
            Ok(())
        } else {
            let error_text = response.text().await.unwrap_or_default();
            anyhow::bail!("GitLab webhook subscription failed: {}", error_text);
        }
    }

    /// Extract GitHub repository path from URL
    fn extract_github_repo_path(&self, url: &str) -> Result<String> {
        let parsed_url = Url::parse(url).context("Invalid GitHub URL")?;

        if !parsed_url.host_str().unwrap_or("").contains("github.com") {
            anyhow::bail!("Not a GitHub URL: {}", url);
        }

        let path = parsed_url
            .path()
            .trim_start_matches('/')
            .trim_end_matches(".git");
        Ok(path.to_string())
    }

    /// Extract GitLab project ID from URL (simplified implementation)
    fn extract_gitlab_project_id(&self, url: &str) -> Result<String> {
        let parsed_url = Url::parse(url).context("Invalid GitLab URL")?;

        if !parsed_url.host_str().unwrap_or("").contains("gitlab.com") {
            anyhow::bail!("Not a GitLab URL: {}", url);
        }

        let path = parsed_url
            .path()
            .trim_start_matches('/')
            .trim_end_matches(".git");
        // URL encode the project path for GitLab API
        Ok(urlencoding::encode(path).to_string())
    }
}

/// Webhook route handlers
pub async fn webhook_post_handler(
    State(app_state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<Value>, StatusCode> {
    let processor = WebhookProcessor::new(WebhookConfig::default());

    // Check if this is a webhook request
    if !processor.is_webhook_request(&headers) {
        warn!("Received non-webhook request at webhook endpoint");
        return Err(StatusCode::BAD_REQUEST);
    }

    // Verify signature if enabled
    if let Err(e) = processor.verify_signature(&headers, &body) {
        error!("Webhook signature verification failed: {}", e);
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Parse the webhook payload
    let webhook_event = match processor.parse_webhook(&headers, &body) {
        Ok(Some(event)) => event,
        Ok(None) => {
            debug!("Webhook event ignored (unsupported event type)");
            return Ok(Json(
                json!({"status": "ignored", "message": "Unsupported event type"}),
            ));
        }
        Err(e) => {
            error!("Failed to parse webhook payload: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Convert to VCS change
    let vcs_change = match processor.webhook_to_vcs_change(&webhook_event) {
        Ok(Some(change)) => change,
        Ok(None) => {
            debug!("No actionable changes in webhook");
            return Ok(Json(
                json!({"status": "no_changes", "message": "No actionable changes"}),
            ));
        }
        Err(e) => {
            error!("Failed to convert webhook to VCS change: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Process the VCS change (update database, trigger rescheduling)
    if let Err(e) = process_vcs_change(&app_state, &vcs_change).await {
        error!("Failed to process VCS change: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Publish real-time event
    if let Err(e) = publish_webhook_event(&app_state, &webhook_event).await {
        warn!("Failed to publish webhook event: {}", e);
    }

    info!("Webhook processed successfully");
    Ok(Json(
        json!({"status": "success", "message": "Webhook processed"}),
    ))
}

/// Webhook documentation handler
pub async fn webhook_get_handler(
    State(app_state): State<AppState>,
) -> Result<Html<String>, StatusCode> {
    let mut context = tera::Context::new();
    context.insert("webhook_endpoint", "/webhook");
    context.insert(
        "supported_platforms",
        &vec!["GitHub", "GitLab", "Gitea", "Gogs", "Launchpad"],
    );

    match app_state.templates.render("webhook.html", &context) {
        Ok(html) => Ok(Html(html)),
        Err(e) => {
            error!("Failed to render webhook template: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Process VCS change by updating database and triggering rescheduling
async fn process_vcs_change(app_state: &AppState, vcs_change: &VcsChange) -> Result<()> {
    match vcs_change {
        VcsChange::Git {
            urls,
            commit_sha,
            branch,
        } => {
            // Update codebase table with new revision information
            for url in urls {
                if let Err(e) = update_codebase_revision(app_state, url, commit_sha, branch).await {
                    warn!("Failed to update codebase for URL {}: {}", url, e);
                    continue;
                }

                // Trigger rescheduling via runner API
                if let Err(e) = trigger_codebase_reschedule(app_state, url).await {
                    warn!("Failed to trigger reschedule for URL {}: {}", url, e);
                }
            }
        }
        VcsChange::Bzr {
            urls,
            revno,
            revid,
            branch,
        } => {
            // Update Bzr codebase with revision information
            for url in urls {
                if let Err(e) =
                    update_bzr_codebase_revision(app_state, url, *revno, revid, branch).await
                {
                    warn!("Failed to update Bzr codebase for URL {}: {}", url, e);
                    continue;
                }

                // Trigger rescheduling via runner API
                if let Err(e) = trigger_codebase_reschedule(app_state, url).await {
                    warn!("Failed to trigger reschedule for URL {}: {}", url, e);
                }
            }
        }
    }
    Ok(())
}

/// Update Git codebase revision in database
async fn update_codebase_revision(
    app_state: &AppState,
    url: &str,
    commit_sha: &str,
    branch: &str,
) -> Result<()> {
    let query = "
        UPDATE codebase 
        SET vcs_last_revision = $1, last_scanned = NOW()
        WHERE branch_url = $2 OR branch_url LIKE $3
    ";

    let url_pattern = format!("%{}%", url);

    sqlx::query(query)
        .bind(commit_sha)
        .bind(url)
        .bind(&url_pattern)
        .execute(app_state.database.pool())
        .await
        .context("Failed to update codebase revision")?;

    debug!(
        "Updated codebase revision for URL {} to {}",
        url, commit_sha
    );
    Ok(())
}

/// Update Bzr codebase revision in database
async fn update_bzr_codebase_revision(
    app_state: &AppState,
    url: &str,
    revno: i32,
    revid: &str,
    branch: &str,
) -> Result<()> {
    let query = "
        UPDATE codebase 
        SET vcs_last_revision = $1, last_scanned = NOW()
        WHERE branch_url = $2 OR branch_url LIKE $3
    ";

    let url_pattern = format!("%{}%", url);

    sqlx::query(query)
        .bind(revid)
        .bind(url)
        .bind(&url_pattern)
        .execute(app_state.database.pool())
        .await
        .context("Failed to update Bzr codebase revision")?;

    debug!(
        "Updated Bzr codebase revision for URL {} to revno {} ({})",
        url, revno, revid
    );
    Ok(())
}

/// Trigger codebase rescheduling via runner API
async fn trigger_codebase_reschedule(app_state: &AppState, url: &str) -> Result<()> {
    // This would typically call the runner service API to trigger rescheduling
    // For now, we'll just log the action
    debug!("Would trigger reschedule for codebase URL: {}", url);

    // TODO: Implement actual runner API call
    // let runner_url = app_state.config.runner_url();
    // let response = app_state.http_client
    //     .post(&format!("{}/api/v1/reschedule", runner_url))
    //     .json(&json!({"codebase_url": url}))
    //     .send()
    //     .await?;

    Ok(())
}

/// Publish webhook event to real-time system
async fn publish_webhook_event(app_state: &AppState, webhook_event: &WebhookEvent) -> Result<()> {
    // Extract relevant information for real-time event
    let (platform, repository_name) = match webhook_event {
        WebhookEvent::GitHub { repository, .. } => ("github", repository.full_name.clone()),
        WebhookEvent::GitLab { project, .. } => ("gitlab", project.path_with_namespace.clone()),
        WebhookEvent::Gitea { repository, .. } => ("gitea", repository.full_name.clone()),
        WebhookEvent::Gogs { repository, .. } => ("gogs", repository.full_name.clone()),
        WebhookEvent::Launchpad {
            bzr_branch,
            git_repository,
            ..
        } => {
            let unknown = "unknown".to_string();
            let repo = bzr_branch
                .as_ref()
                .or(git_repository.as_ref())
                .unwrap_or(&unknown);
            ("launchpad", repo.clone())
        }
    };

    // Publish via realtime manager
    app_state
        .realtime
        .publish_campaign_update(
            "webhook".to_string(),
            "push_received".to_string(),
            json!({
                "platform": platform,
                "repository": repository_name,
                "timestamp": chrono::Utc::now(),
            }),
        )
        .await
        .context("Failed to publish webhook real-time event")?;

    Ok(())
}

/// Create webhook routes
pub fn create_webhook_routes() -> Router<AppState> {
    Router::new().route(
        "/webhook",
        get(webhook_get_handler).post(webhook_post_handler),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_branch_from_ref() {
        let processor = WebhookProcessor::new(WebhookConfig::default());

        assert_eq!(processor.extract_branch_from_ref("refs/heads/main"), "main");
        assert_eq!(
            processor.extract_branch_from_ref("refs/heads/develop"),
            "develop"
        );
        assert_eq!(processor.extract_branch_from_ref("main"), "main");
    }

    #[test]
    fn test_github_repo_path_extraction() {
        let registration = WebhookRegistration::new();

        assert_eq!(
            registration
                .extract_github_repo_path("https://github.com/owner/repo")
                .unwrap(),
            "owner/repo"
        );
        assert_eq!(
            registration
                .extract_github_repo_path("https://github.com/owner/repo.git")
                .unwrap(),
            "owner/repo"
        );
    }

    #[test]
    fn test_gitlab_project_id_extraction() {
        let registration = WebhookRegistration::new();

        assert_eq!(
            registration
                .extract_gitlab_project_id("https://gitlab.com/group/project")
                .unwrap(),
            "group%2Fproject"
        );
    }

    #[tokio::test]
    async fn test_webhook_event_serialization() {
        let event = WebhookEvent::GitHub {
            event_type: "push".to_string(),
            repository: GitHubRepository {
                name: "test-repo".to_string(),
                full_name: "owner/test-repo".to_string(),
                html_url: "https://github.com/owner/test-repo".to_string(),
                clone_url: "https://github.com/owner/test-repo.git".to_string(),
                ssh_url: "git@github.com:owner/test-repo.git".to_string(),
                git_url: "git://github.com/owner/test-repo.git".to_string(),
                default_branch: "main".to_string(),
            },
            commits: vec![],
            head_commit: None,
            git_ref: "refs/heads/main".to_string(),
        };

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: WebhookEvent = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            WebhookEvent::GitHub { event_type, .. } => {
                assert_eq!(event_type, "push");
            }
            _ => panic!("Expected GitHub event"),
        }
    }
}
