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
}
