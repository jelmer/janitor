use crate::database::RunnerDatabase;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::error::Error;
use std::fmt;

/// Information about a run that can be resumed from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeInfo {
    /// ID of the run to resume from.
    pub run_id: String,
    /// Campaign name.
    pub campaign: String,
    /// Codebase URL.
    pub codebase: String,
    /// Branch name.
    pub branch_name: String,
    /// Result code of the completed run.
    pub result_code: String,
    /// Revision ID if available.
    pub revision: Option<String>,
}

/// Errors that can occur during resume operations.
#[derive(Debug)]
pub enum ResumeError {
    /// Database operation failed.
    DatabaseError(sqlx::Error),
    /// No resume information found.
    NoResumeFound,
    /// Invalid resume branch.
    InvalidResumeBranch(String),
    /// VCS operation failed.
    VcsError(String),
}

impl fmt::Display for ResumeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResumeError::DatabaseError(e) => write!(f, "Database error: {}", e),
            ResumeError::NoResumeFound => write!(f, "No suitable resume found"),
            ResumeError::InvalidResumeBranch(msg) => write!(f, "Invalid resume branch: {}", msg),
            ResumeError::VcsError(msg) => write!(f, "VCS error: {}", msg),
        }
    }
}

impl Error for ResumeError {}

impl From<sqlx::Error> for ResumeError {
    fn from(error: sqlx::Error) -> Self {
        ResumeError::DatabaseError(error)
    }
}

/// Service for managing resume logic for interrupted runs.
pub struct ResumeService {
    database: RunnerDatabase,
}

impl ResumeService {
    /// Create a new resume service.
    pub fn new(database: RunnerDatabase) -> Self {
        Self { database }
    }

    /// Check if a resume result exists for the given campaign and branch
    pub async fn check_resume_result(
        &self,
        campaign: &str,
        branch_name: &str,
    ) -> Result<Option<ResumeInfo>, ResumeError> {
        let row = sqlx::query(
            r#"
            SELECT r.id, r.campaign, r.codebase, r.branch_name, r.result_code, r.revision
            FROM run r
            WHERE r.campaign = $1 
            AND r.branch_name = $2 
            AND r.result_code IN ('success', 'nothing-new-to-do')
            AND r.finish_time IS NOT NULL
            ORDER BY r.finish_time DESC
            LIMIT 1
            "#,
        )
        .bind(campaign)
        .bind(branch_name)
        .fetch_optional(self.database.pool())
        .await?;

        if let Some(row) = row {
            Ok(Some(ResumeInfo {
                run_id: row.get("id"),
                campaign: row.get("campaign"),
                codebase: row.get("codebase"),
                branch_name: row.get("branch_name"),
                result_code: row.get("result_code"),
                revision: row.get("revision"),
            }))
        } else {
            Ok(None)
        }
    }

    /// Find an existing resume branch on the VCS forge
    pub async fn open_resume_branch(
        &self,
        codebase_url: &str,
        campaign: &str,
    ) -> Result<Option<String>, ResumeError> {
        // Extract the base name from campaign for branch naming
        let branch_prefix = self.get_branch_prefix(campaign);

        // For now, we'll implement a simple pattern-based search
        // In a full implementation, this would query the actual VCS forge API
        let potential_branches = vec![
            format!("{}-proposed", branch_prefix),
            format!("{}/proposed", branch_prefix),
            format!("debian/{}-proposed", branch_prefix),
            format!("proposed/{}", branch_prefix),
        ];

        // Check if any of these branches exist in our database as successful runs
        for branch_name in potential_branches {
            if let Some(_resume_info) = self.check_resume_result(campaign, &branch_name).await? {
                return Ok(Some(branch_name));
            }
        }

        Ok(None)
    }

    /// Set resume information for a run
    pub async fn set_resume_from(
        &self,
        run_id: &str,
        resume_from_id: &str,
    ) -> Result<(), ResumeError> {
        sqlx::query(
            r#"
            UPDATE run 
            SET resume_from = $1 
            WHERE id = $2
            "#,
        )
        .bind(resume_from_id)
        .bind(run_id)
        .execute(self.database.pool())
        .await?;

        Ok(())
    }

    /// Check if a run can be resumed from another run
    pub async fn can_resume_from(
        &self,
        run_id: &str,
        potential_resume_id: &str,
    ) -> Result<bool, ResumeError> {
        let row = sqlx::query(
            r#"
            SELECT 
                r1.campaign as current_campaign,
                r1.codebase as current_codebase,
                r2.campaign as resume_campaign,
                r2.codebase as resume_codebase,
                r2.result_code as resume_result
            FROM run r1, run r2
            WHERE r1.id = $1 AND r2.id = $2
            "#,
        )
        .bind(run_id)
        .bind(potential_resume_id)
        .fetch_optional(self.database.pool())
        .await?;

        if let Some(row) = row {
            let current_campaign: String = row.get("current_campaign");
            let current_codebase: String = row.get("current_codebase");
            let resume_campaign: String = row.get("resume_campaign");
            let resume_codebase: String = row.get("resume_codebase");
            let resume_result: String = row.get("resume_result");

            // Can only resume if campaigns and codebases match and previous run was successful
            Ok(current_campaign == resume_campaign
                && current_codebase == resume_codebase
                && (resume_result == "success" || resume_result == "nothing-new-to-do"))
        } else {
            Ok(false)
        }
    }

    /// Get all runs that resume from a specific run
    pub async fn get_resume_descendants(&self, run_id: &str) -> Result<Vec<String>, ResumeError> {
        let rows = sqlx::query(
            r#"
            WITH RECURSIVE resume_tree AS (
                SELECT id, resume_from, 1 as level
                FROM run
                WHERE resume_from = $1
                
                UNION ALL
                
                SELECT r.id, r.resume_from, rt.level + 1
                FROM run r
                INNER JOIN resume_tree rt ON r.resume_from = rt.id
                WHERE rt.level < 10  -- Prevent infinite recursion
            )
            SELECT id FROM resume_tree ORDER BY level, id
            "#,
        )
        .bind(run_id)
        .fetch_all(self.database.pool())
        .await?;

        Ok(rows.into_iter().map(|row| row.get("id")).collect())
    }

    /// Get the resume chain for a run (all the runs it transitively resumes from)
    pub async fn get_resume_chain(&self, run_id: &str) -> Result<Vec<String>, ResumeError> {
        let rows = sqlx::query(
            r#"
            WITH RECURSIVE resume_chain AS (
                SELECT id, resume_from, 0 as level
                FROM run
                WHERE id = $1
                
                UNION ALL
                
                SELECT r.id, r.resume_from, rc.level + 1
                FROM run r
                INNER JOIN resume_chain rc ON r.id = rc.resume_from
                WHERE rc.level < 10  -- Prevent infinite recursion
            )
            SELECT id FROM resume_chain WHERE id != $1 ORDER BY level DESC
            "#,
        )
        .bind(run_id)
        .fetch_all(self.database.pool())
        .await?;

        Ok(rows.into_iter().map(|row| row.get("id")).collect())
    }

    /// Validate that resume relationships are consistent
    pub async fn validate_resume_consistency(&self) -> Result<Vec<String>, ResumeError> {
        let rows = sqlx::query(
            r#"
            SELECT r1.id, r1.resume_from
            FROM run r1
            LEFT JOIN run r2 ON r1.resume_from = r2.id
            WHERE r1.resume_from IS NOT NULL 
            AND (r2.id IS NULL OR r2.result_code NOT IN ('success', 'nothing-new-to-do'))
            "#,
        )
        .fetch_all(self.database.pool())
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let id: String = row.get("id");
                let resume_from: Option<String> = row.get("resume_from");
                format!("Run {} has invalid resume_from: {:?}", id, resume_from)
            })
            .collect())
    }

    fn get_branch_prefix(&self, campaign: &str) -> String {
        // Extract meaningful part of campaign name for branch naming
        // This matches the Python logic for branch name generation
        if let Some(last_slash) = campaign.rfind('/') {
            campaign[last_slash + 1..].to_string()
        } else {
            campaign.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resume_info_serialization() {
        let resume_info = ResumeInfo {
            run_id: "test-run-1".to_string(),
            campaign: "test-campaign".to_string(),
            codebase: "https://example.com/repo".to_string(),
            branch_name: "test-branch".to_string(),
            result_code: "success".to_string(),
            revision: Some("abc123".to_string()),
        };

        let json = serde_json::to_string(&resume_info).unwrap();
        let deserialized: ResumeInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(resume_info.run_id, deserialized.run_id);
        assert_eq!(resume_info.campaign, deserialized.campaign);
    }

    #[test]
    #[ignore] // Skip this test as it requires a real database
    fn test_branch_prefix_extraction() {
        // let service = ResumeService::new(RunnerDatabase::new(...));

        // assert_eq!(service.get_branch_prefix("simple-campaign"), "simple-campaign");
        // assert_eq!(service.get_branch_prefix("namespace/campaign-name"), "campaign-name");
        // assert_eq!(service.get_branch_prefix("deep/nested/campaign"), "campaign");
    }

    #[test]
    fn test_resume_error_display() {
        let error = ResumeError::NoResumeFound;
        assert_eq!(error.to_string(), "No suitable resume found");

        let error = ResumeError::InvalidResumeBranch("test".to_string());
        assert_eq!(error.to_string(), "Invalid resume branch: test");
    }
}
