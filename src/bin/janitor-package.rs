//! CLI for the actions a logged-in, non-admin janitor user can take.
//!
//! Talks to a running janitor site instance over its public HTTP API, and
//! covers the maintainer/QA-reviewer side of the web UI: triggering
//! publishes, submitting reviews, rescheduling and inspecting runs, and
//! browsing merge proposals. Admin-gated actions live in `janitor-admin`
//! instead.

use clap::{Parser, Subcommand, ValueEnum};
use janitor::site_client::{expect_success, ApiClient};
use serde_json::Value;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    version,
    about = "Act as a logged-in janitor user over the site's HTTP API"
)]
struct Cli {
    /// Base URL of the janitor site (e.g. https://janitor.debian.net).
    #[arg(
        long,
        env = "JANITOR_URL",
        default_value = "https://janitor.debian.net"
    )]
    url: reqwest::Url,

    /// Username for HTTP basic auth.
    #[arg(long, env = "JANITOR_USER")]
    user: Option<String>,

    /// Password for HTTP basic auth.
    #[arg(long, env = "JANITOR_PASSWORD")]
    password: Option<String>,

    #[command(subcommand)]
    command: Command,

    #[command(flatten)]
    logging: janitor::logging::LoggingArgs,
}

#[derive(Subcommand)]
enum Command {
    /// Trigger and inspect publish attempts.
    Publish {
        #[command(subcommand)]
        cmd: PublishCmd,
    },
    /// Submit QA review verdicts and browse the review queue.
    Review {
        #[command(subcommand)]
        cmd: ReviewCmd,
    },
}

#[derive(Subcommand)]
enum PublishCmd {
    /// Trigger a publish attempt for a codebase.
    Trigger {
        /// Campaign name (e.g. lintian-fixes).
        campaign: String,
        /// Codebase name.
        codebase: String,
        /// Publish mode. Leave unset to use the codebase's configured mode.
        #[arg(long)]
        mode: Option<PublishMode>,
    },
    /// Show details of a single publish attempt.
    Show {
        /// Publish ID.
        publish_id: String,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum PublishMode {
    PushDerived,
    Push,
    Propose,
    AttemptPush,
}

impl PublishMode {
    fn as_str(&self) -> &'static str {
        match self {
            PublishMode::PushDerived => "push-derived",
            PublishMode::Push => "push",
            PublishMode::Propose => "propose",
            PublishMode::AttemptPush => "attempt-push",
        }
    }
}

#[derive(Subcommand)]
enum ReviewCmd {
    /// Submit a review verdict for a run.
    Submit {
        /// Run ID being reviewed.
        run_id: String,
        /// Review verdict.
        verdict: Verdict,
        /// Optional free-text comment to attach to the review.
        #[arg(long)]
        comment: Option<String>,
        /// Restrict the regenerated review queue to publishable runs only
        /// (defaults to true, matching the web UI).
        #[arg(long)]
        publishable_only: Option<bool>,
        /// Restrict the regenerated review queue to these campaigns. May be
        /// given more than once. Only affects the (discarded) HTML the
        /// server renders after storing the review, not the verdict itself.
        #[arg(long = "suite")]
        suites: Vec<String>,
    },
    /// List runs waiting for review.
    NeedsReview {
        /// Restrict to a single campaign.
        #[arg(long)]
        campaign: Option<String>,
        /// Reviewer email to check per-reviewer review state for. Defaults
        /// to the authenticated user.
        #[arg(long)]
        reviewer: Option<String>,
        /// Only include runs that are ready to be published (default true).
        #[arg(long)]
        publishable_only: Option<bool>,
        /// Only include runs where a review is required.
        #[arg(long)]
        required_only: Option<bool>,
        /// Maximum number of runs to list.
        #[arg(long, default_value_t = 200)]
        limit: u32,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Verdict {
    Approve,
    Reject,
    Reschedule,
    Abstain,
}

impl Verdict {
    fn as_str(&self) -> &'static str {
        match self {
            Verdict::Approve => "approve",
            Verdict::Reject => "reject",
            Verdict::Reschedule => "reschedule",
            Verdict::Abstain => "abstain",
        }
    }
}

#[derive(Debug, serde::Deserialize)]
struct NeedsReviewEntry {
    id: String,
    codebase: String,
    campaign: String,
}

async fn cmd_publish_trigger(
    client: &ApiClient,
    campaign: &str,
    codebase: &str,
    mode: Option<PublishMode>,
) -> Result<(), String> {
    let path = format!("api/{}/c/{}/publish", campaign, codebase);
    let mut form: Vec<(&str, &str)> = Vec::new();
    if let Some(mode) = mode {
        form.push(("mode", mode.as_str()));
    }
    let resp = client
        .request(reqwest::Method::POST, &path)?
        .form(&form)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    // The site forwards the publisher's own success/failure JSON as-is, so a
    // non-2xx status here still carries a useful JSON body rather than plain
    // text; print it either way instead of using expect_success.
    let status = resp.status();
    let body: Value = resp.json().await.map_err(|e| e.to_string())?;
    println!("{}", serde_json::to_string_pretty(&body).unwrap());
    if !status.is_success() {
        return Err(format!("publish request failed: HTTP {}", status));
    }
    Ok(())
}

async fn cmd_publish_show(client: &ApiClient, publish_id: &str) -> Result<(), String> {
    let path = format!("api/publish/{}", publish_id);
    let resp = client
        .request(reqwest::Method::GET, &path)?
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let resp = expect_success(resp).await?;
    let body: Value = resp.json().await.map_err(|e| e.to_string())?;
    println!("{}", serde_json::to_string_pretty(&body).unwrap());
    Ok(())
}

async fn cmd_review_submit(
    client: &ApiClient,
    run_id: &str,
    verdict: Verdict,
    comment: Option<&str>,
    publishable_only: Option<bool>,
    suites: &[String],
) -> Result<(), String> {
    let mut form: Vec<(&str, String)> = Vec::new();
    form.push(("run_id", run_id.to_string()));
    form.push(("verdict", verdict.as_str().to_string()));
    if let Some(comment) = comment {
        form.push(("review_comment", comment.to_string()));
    }
    if let Some(publishable_only) = publishable_only {
        form.push(("publishable_only", publishable_only.to_string()));
    }
    for suite in suites {
        form.push(("suite", suite.clone()));
    }
    let resp = client
        .request(reqwest::Method::POST, "cupboard/review")?
        .form(&form)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    // The endpoint re-renders the (now updated) review queue as HTML rather
    // than returning JSON, so there's nothing structured to print back.
    expect_success(resp).await?;
    println!("Submitted verdict {} for run {}", verdict.as_str(), run_id);
    Ok(())
}

async fn cmd_review_needs_review(
    client: &ApiClient,
    campaign: Option<&str>,
    reviewer: Option<&str>,
    publishable_only: Option<bool>,
    required_only: Option<bool>,
    limit: u32,
) -> Result<(), String> {
    let path = match campaign {
        Some(campaign) => format!("cupboard/api/{}/needs-review", campaign),
        None => "cupboard/api/needs-review".to_string(),
    };
    let mut query: Vec<(&str, String)> = vec![("limit", limit.to_string())];
    if let Some(reviewer) = reviewer {
        query.push(("reviewer", reviewer.to_string()));
    }
    if let Some(publishable_only) = publishable_only {
        query.push(("publishable_only", publishable_only.to_string()));
    }
    if let Some(required_only) = required_only {
        query.push(("required_only", required_only.to_string()));
    }
    let resp = client
        .request(reqwest::Method::GET, &path)?
        .query(&query)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let resp = expect_success(resp).await?;
    let entries: Vec<NeedsReviewEntry> = resp.json().await.map_err(|e| e.to_string())?;
    if entries.is_empty() {
        println!("No runs waiting for review.");
        return Ok(());
    }
    println!("{:<38} {:<24} RUN ID", "CODEBASE", "CAMPAIGN");
    for entry in entries {
        println!("{:<38} {:<24} {}", entry.codebase, entry.campaign, entry.id);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    cli.logging.init();

    let client = match ApiClient::new(cli.url, cli.user, cli.password) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("failed to build HTTP client: {}", e);
            return ExitCode::from(2);
        }
    };

    let result = match cli.command {
        Command::Publish { cmd } => match cmd {
            PublishCmd::Trigger {
                campaign,
                codebase,
                mode,
            } => cmd_publish_trigger(&client, &campaign, &codebase, mode).await,
            PublishCmd::Show { publish_id } => cmd_publish_show(&client, &publish_id).await,
        },
        Command::Review { cmd } => match cmd {
            ReviewCmd::Submit {
                run_id,
                verdict,
                comment,
                publishable_only,
                suites,
            } => {
                cmd_review_submit(
                    &client,
                    &run_id,
                    verdict,
                    comment.as_deref(),
                    publishable_only,
                    &suites,
                )
                .await
            }
            ReviewCmd::NeedsReview {
                campaign,
                reviewer,
                publishable_only,
                required_only,
                limit,
            } => {
                cmd_review_needs_review(
                    &client,
                    campaign.as_deref(),
                    reviewer.as_deref(),
                    publishable_only,
                    required_only,
                    limit,
                )
                .await
            }
        },
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_publish_trigger_with_mode() {
        let cli = Cli::try_parse_from([
            "janitor-package",
            "--url",
            "https://example.com",
            "publish",
            "trigger",
            "lintian-fixes",
            "mypkg",
            "--mode",
            "propose",
        ])
        .unwrap();
        match cli.command {
            Command::Publish {
                cmd: PublishCmd::Trigger {
                    campaign,
                    codebase,
                    mode,
                },
            } => {
                assert_eq!(campaign, "lintian-fixes");
                assert_eq!(codebase, "mypkg");
                assert_eq!(mode.unwrap().as_str(), "propose");
            }
            _ => panic!("wrong subcommand"),
        }
    }

    #[test]
    fn cli_parses_publish_trigger_with_multiword_mode() {
        // clap's derived ValueEnum parsing is kebab-case, so the two-word
        // enum variant must be spelled with a hyphen on the command line.
        let cli = Cli::try_parse_from([
            "janitor-package",
            "publish",
            "trigger",
            "lintian-fixes",
            "mypkg",
            "--mode",
            "attempt-push",
        ])
        .unwrap();
        match cli.command {
            Command::Publish {
                cmd: PublishCmd::Trigger { mode, .. },
            } => {
                assert_eq!(mode.unwrap().as_str(), "attempt-push");
            }
            _ => panic!("wrong subcommand"),
        }
    }

    #[test]
    fn cli_parses_publish_trigger_without_mode() {
        let cli = Cli::try_parse_from([
            "janitor-package",
            "publish",
            "trigger",
            "lintian-fixes",
            "mypkg",
        ])
        .unwrap();
        match cli.command {
            Command::Publish {
                cmd: PublishCmd::Trigger { mode, .. },
            } => {
                assert!(mode.is_none());
            }
            _ => panic!("wrong subcommand"),
        }
    }

    #[test]
    fn cli_parses_publish_show() {
        let cli = Cli::try_parse_from(["janitor-package", "publish", "show", "1234"]).unwrap();
        match cli.command {
            Command::Publish {
                cmd: PublishCmd::Show { publish_id },
            } => {
                assert_eq!(publish_id, "1234");
            }
            _ => panic!("wrong subcommand"),
        }
    }

    #[test]
    fn cli_parses_review_submit_with_comment_and_suites() {
        let cli = Cli::try_parse_from([
            "janitor-package",
            "review",
            "submit",
            "a1b2c3d4-5f6a-4b3c-9d8e-1234567890ab",
            "reschedule",
            "--comment",
            "flaky build, please retry",
            "--suite",
            "lintian-fixes",
            "--suite",
            "multiarch-hints",
        ])
        .unwrap();
        match cli.command {
            Command::Review {
                cmd:
                    ReviewCmd::Submit {
                        run_id,
                        verdict,
                        comment,
                        publishable_only,
                        suites,
                    },
            } => {
                assert_eq!(run_id, "a1b2c3d4-5f6a-4b3c-9d8e-1234567890ab");
                assert_eq!(verdict.as_str(), "reschedule");
                assert_eq!(comment.as_deref(), Some("flaky build, please retry"));
                assert!(publishable_only.is_none());
                assert_eq!(suites, vec!["lintian-fixes", "multiarch-hints"]);
            }
            _ => panic!("wrong subcommand"),
        }
    }

    #[test]
    fn cli_parses_review_needs_review_defaults() {
        let cli = Cli::try_parse_from(["janitor-package", "review", "needs-review"]).unwrap();
        match cli.command {
            Command::Review {
                cmd:
                    ReviewCmd::NeedsReview {
                        campaign,
                        reviewer,
                        publishable_only,
                        required_only,
                        limit,
                    },
            } => {
                assert!(campaign.is_none());
                assert!(reviewer.is_none());
                assert!(publishable_only.is_none());
                assert!(required_only.is_none());
                assert_eq!(limit, 200);
            }
            _ => panic!("wrong subcommand"),
        }
    }

    #[test]
    fn cli_parses_review_needs_review_with_filters() {
        let cli = Cli::try_parse_from([
            "janitor-package",
            "review",
            "needs-review",
            "--campaign",
            "lintian-fixes",
            "--publishable-only",
            "false",
            "--required-only",
            "true",
            "--limit",
            "10",
        ])
        .unwrap();
        match cli.command {
            Command::Review {
                cmd:
                    ReviewCmd::NeedsReview {
                        campaign,
                        publishable_only,
                        required_only,
                        limit,
                        ..
                    },
            } => {
                assert_eq!(campaign.as_deref(), Some("lintian-fixes"));
                assert_eq!(publishable_only, Some(false));
                assert_eq!(required_only, Some(true));
                assert_eq!(limit, 10);
            }
            _ => panic!("wrong subcommand"),
        }
    }
}
