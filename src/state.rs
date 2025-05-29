use crate::config::Config;
use breezyshim::RevisionId;
use log::warn;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, Postgres};
use sqlx::{PgPool, Pool};

/// Create a connection pool to the database
///
/// # Arguments
/// * `config` - The configuration to use for the database connection
///
/// # Returns
/// A connection pool to the database
pub async fn create_pool(config: &Config) -> Result<Pool<Postgres>, sqlx::Error> {
    let pool_options = PgPoolOptions::new().max_connections(5);
    let pool = if let Some(ref database_url) = config.database_location {
        pool_options.connect(database_url).await?
    } else {
        let options = PgConnectOptions::new();
        pool_options.connect_with(options).await?
    };

    Ok(pool)
}

#[derive(Debug, Clone, Eq, sqlx::FromRow)]
pub struct Run {
    pub id: String,
    pub command: String,
    pub description: Option<String>,
    pub result_code: String,
    pub main_branch_revision: Option<RevisionId>,
    pub revision: Option<RevisionId>,
    pub context: Option<String>,
    pub result: Option<serde_json::Value>,
    pub suite: String,
    pub instigated_context: Option<String>,
    pub vcs_type: String,
    pub branch_url: String,
    pub logfilenames: Option<Vec<String>>,
    pub worker_name: Option<String>,
    pub result_branches: Option<Vec<(String, String, Option<RevisionId>, Option<RevisionId>)>>,
    pub result_tags: Option<Vec<(String, String)>>,
    pub target_branch_url: Option<String>,
    pub change_set: String,
    pub failure_details: Option<serde_json::Value>,
    pub failure_transient: Option<bool>,
    pub failure_stage: Option<String>,
    pub codebase: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub finish_time: chrono::DateTime<chrono::Utc>,
    pub value: Option<i32>,
}

impl PartialEq for Run {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Run {
    pub fn duration(&self) -> chrono::Duration {
        self.finish_time - self.start_time
    }

    pub fn get_result_branch(
        &self,
        role: &str,
    ) -> Option<(String, Option<RevisionId>, Option<RevisionId>)> {
        self.result_branches.as_ref().and_then(|branches| {
            branches
                .iter()
                .find(|(r, _, _, _)| r == role)
                .map(|(_, n, br, r)| (n.clone(), r.clone(), br.clone()))
        })
    }

    /// Alias for suite field (matches Python API)
    pub fn campaign(&self) -> &str {
        &self.suite
    }

    // Note: Manual from_row() method removed - the struct derives sqlx::FromRow
    // which provides automatic row mapping for standard types
}

impl PartialOrd for Run {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Run {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Match Python's __lt__ method - compare by start_time primarily
        self.start_time.cmp(&other.start_time)
    }
}

/// Check if a codebase has cotenants (other codebases with the same URL)
///
/// # Arguments
/// * `conn` - Database connection pool
/// * `codebase` - The codebase to check
/// * `url` - The URL to check for other codebases
///
/// # Returns
/// Some(true) if there are cotenants, Some(false) if not, None if single codebase
pub async fn has_cotenants(
    conn: &PgPool,
    codebase: &str,
    url: &url::Url,
) -> Result<Option<bool>, sqlx::Error> {
    #[derive(Debug, Clone, sqlx::FromRow)]
    struct Codebase {
        pub name: String,
    }
    let url = breezyshim::urlutils::split_segment_parameters(url)
        .0
        .to_string();

    let rows: Vec<Codebase> =
        sqlx::query_as("SELECT name FROM codebase where branch_url = $1 or url = $1")
            .bind(url.trim_end_matches('/'))
            .fetch_all(conn)
            .await?;

    Ok(match rows.len() {
        0 => {
            // Uhm, we actually don't really know
            warn!(
                "Unable to figure out if {} has cotenants on {}",
                codebase, url
            );
            None
        }
        1 => Some(rows[0].name != codebase),
        _ => Some(true),
    })
}

/// Iterate over publishable suites for a given codebase
///
/// # Arguments
/// * `conn` - Database connection pool
/// * `codebase` - The codebase to check for publishable suites
///
/// # Returns
/// A list of suite names that are ready for publishing
pub async fn iter_publishable_suites(
    conn: &PgPool,
    codebase: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT DISTINCT suite FROM publish_ready WHERE codebase = $1")
            .bind(codebase)
            .fetch_all(conn)
            .await?;

    Ok(rows.into_iter().map(|row| row.0).collect::<Vec<_>>())
}

/// Get the result branch information for a specific role
///
/// This is a standalone utility function that matches the Python API
///
/// # Arguments
/// * `result_branches` - The result branches array from a Run
/// * `role` - The role to search for
///
/// # Returns
/// Tuple of (name, base_revision, revision) if found
///
/// # Errors
/// Returns an error if the role is not found
pub fn get_result_branch(
    result_branches: &[(String, String, Option<RevisionId>, Option<RevisionId>)],
    role: &str,
) -> Result<(String, Option<RevisionId>, Option<RevisionId>), String> {
    result_branches
        .iter()
        .find(|(r, _, _, _)| r == role)
        .map(|(_, n, br, r)| (n.clone(), r.clone(), br.clone()))
        .ok_or_else(|| format!("Role '{}' not found in result branches", role))
}
