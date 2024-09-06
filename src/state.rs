use crate::config::Config;
use breezyshim::RevisionId;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, Postgres};
use sqlx::Pool;

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

#[derive(Debug, Clone, sqlx::FromRow)]
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
}
