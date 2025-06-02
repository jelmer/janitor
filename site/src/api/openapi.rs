use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};

use super::schemas::*;

/// OpenAPI documentation for the Janitor API
#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::routes::health_check,
        crate::api::routes::api_status,
        crate::api::routes::get_queue_status,
        crate::api::routes::get_active_runs,
        crate::api::routes::get_active_run,
        crate::api::routes::get_run_logs,
        crate::api::routes::get_run_log_file,
        crate::api::routes::get_run_diff,
        crate::api::routes::get_run_debdiff,
        crate::api::routes::get_run_diffoscope,
        crate::api::routes::get_merge_proposals,
        crate::api::routes::get_runner_status,
    ),
    components(
        schemas(
            ScheduleResult,
            MergeProposal,
            QueueItem,
            BuildInfo,
            Run,
            ResultBranch,
            ResultTag,
            WorkerResult,
            PublishMode,
            PublishRequest,
            RescheduleRequest,
            MassRescheduleRequest,
            LogFile,
            DiffInfo,
            UserInfo,
            CampaignStatus,
            HealthStatus,
            ServiceHealth,
            super::types::ApiResponse<serde_json::Value>,
            super::types::ApiError,
            super::types::PaginationInfo,
            super::types::CommonQuery,
            super::types::QueueStatus,
        )
    ),
    tags(
        (name = "health", description = "Health check and status endpoints"),
        (name = "queue", description = "Queue management operations"),
        (name = "runs", description = "Run management and monitoring"),
        (name = "logs", description = "Log file access and streaming"),
        (name = "diffs", description = "Diff generation and access"),
        (name = "merge-proposals", description = "Merge proposal management"),
        (name = "publishing", description = "Publishing operations"),
        (name = "admin", description = "Administrative operations"),
    ),
    modifiers(&SecurityAddon),
    info(
        title = "Janitor API",
        version = "1.0.0",
        description = "REST API for the Janitor automated VCS change management platform",
        contact(
            name = "Jelmer Vernooĳ",
            email = "jelmer@jelmer.uk",
            url = "https://github.com/jelmer/janitor"
        ),
        license(
            name = "GPL-3.0+",
            url = "https://www.gnu.org/licenses/gpl-3.0.html"
        )
    )
)]
pub struct ApiDoc;

/// Security configuration for OpenAPI
pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "cookie_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .description(Some("Session-based authentication via cookies"))
                        .build(),
                ),
            );
        }
    }
}

/// Generate OpenAPI specification as JSON
pub fn generate_openapi_spec() -> String {
    serde_json::to_string_pretty(&ApiDoc::openapi()).unwrap_or_else(|e| {
        tracing::error!("Failed to serialize OpenAPI spec: {}", e);
        "{}".to_string()
    })
}

/// Common OpenAPI response types
pub mod responses {
    use super::*;
    use utoipa::ToSchema;

    /// Standard success response
    #[derive(ToSchema)]
    pub struct SuccessResponse {
        pub data: serde_json::Value,
    }

    /// Standard error response
    #[derive(ToSchema)]
    pub struct ErrorResponse {
        pub error: String,
        pub reason: Option<String>,
        pub details: Option<serde_json::Value>,
    }

    /// Paginated response
    #[derive(ToSchema)]
    pub struct PaginatedResponse<T> {
        pub data: Vec<T>,
        pub pagination: super::super::types::PaginationInfo,
    }

    /// Health check response
    #[derive(ToSchema)]
    pub struct HealthCheckResponse {
        pub status: String,
        pub timestamp: chrono::DateTime<chrono::Utc>,
        pub services: std::collections::HashMap<String, ServiceHealth>,
    }
}

/// Common OpenAPI parameter types
pub mod parameters {
    use utoipa::{IntoParams, ToSchema};

    /// Path parameters for run operations
    #[derive(IntoParams)]
    pub struct RunPathParams {
        /// Run identifier
        #[param(example = "12345")]
        pub run_id: String,
    }

    /// Path parameters for codebase operations
    #[derive(IntoParams)]
    pub struct CodebasePathParams {
        /// Campaign name
        #[param(example = "lintian-fixes")]
        pub campaign: String,

        /// Codebase name
        #[param(example = "package-name")]
        pub codebase: String,
    }

    /// Common query parameters
    #[derive(IntoParams)]
    pub struct CommonQueryParams {
        /// Page offset
        #[param(example = 0)]
        pub offset: Option<i64>,

        /// Page size
        #[param(example = 50)]
        pub limit: Option<i64>,

        /// Search query
        #[param(example = "search term")]
        pub search: Option<String>,

        /// Sort field
        #[param(example = "created_time")]
        pub sort: Option<String>,

        /// Sort order
        #[param(example = "desc")]
        pub order: Option<String>,
    }

    /// Log file path parameters
    #[derive(IntoParams)]
    pub struct LogFileParams {
        /// Run identifier
        #[param(example = "12345")]
        pub run_id: String,

        /// Log filename
        #[param(example = "build.log")]
        pub filename: String,
    }
}

/// Example data for OpenAPI documentation
pub mod examples {
    use super::*;
    use chrono::Utc;
    use std::collections::HashMap;

    /// Example schedule result
    pub fn example_schedule_result() -> ScheduleResult {
        ScheduleResult {
            codebase: "example-package".to_string(),
            campaign: "lintian-fixes".to_string(),
            offset: Some(10),
            estimated_duration_seconds: Some(300),
            queue_position: 5,
            queue_wait_time: 1200,
        }
    }

    /// Example run information
    pub fn example_run() -> Run {
        Run {
            run_id: "run-12345".to_string(),
            start_time: Some(Utc::now()),
            finish_time: Some(Utc::now()),
            command: "lintian-brush".to_string(),
            description: Some("Fix lintian warnings".to_string()),
            build_info: Some(BuildInfo {
                version: Some("1.0.0".to_string()),
                distribution: Some("unstable".to_string()),
                architecture: Some("amd64".to_string()),
                status: Some("success".to_string()),
            }),
            result_code: Some("success".to_string()),
            main_branch_revision: Some("abc123".to_string()),
            revision: Some("def456".to_string()),
            context: Some(serde_json::json!({"branch": "main"})),
            suite: Some("unstable".to_string()),
            vcs_type: Some("git".to_string()),
            branch_url: Some("https://github.com/example/package".to_string()),
            logfilenames: vec!["build.log".to_string(), "test.log".to_string()],
            worker_name: Some("worker-01".to_string()),
            result_branches: vec![ResultBranch {
                name: "main".to_string(),
                role: Some("main".to_string()),
                base_revision: Some("abc123".to_string()),
                revision: Some("def456".to_string()),
            }],
            result_tags: vec![],
            target_branch_url: Some("https://github.com/example/package".to_string()),
            change_set: Some("lintian-fixes".to_string()),
            failure_transient: Some(false),
            failure_stage: None,
            codebase: "example-package".to_string(),
            campaign: "lintian-fixes".to_string(),
            subpath: None,
        }
    }

    /// Example merge proposal
    pub fn example_merge_proposal() -> MergeProposal {
        MergeProposal {
            url: "https://github.com/example/package/pull/123".to_string(),
            status: "open".to_string(),
            codebase: Some("example-package".to_string()),
            campaign: Some("lintian-fixes".to_string()),
            description: Some("Fix lintian warnings".to_string()),
            created_time: Some(Utc::now()),
            updated_time: Some(Utc::now()),
            merged_at: None,
            run_id: Some("run-12345".to_string()),
        }
    }

    /// Example health status
    pub fn example_health_status() -> HealthStatus {
        let mut services = HashMap::new();
        services.insert(
            "database".to_string(),
            ServiceHealth {
                status: "healthy".to_string(),
                error: None,
                response_time_ms: Some(15),
                last_check: Some(Utc::now()),
            },
        );
        services.insert(
            "redis".to_string(),
            ServiceHealth {
                status: "healthy".to_string(),
                error: None,
                response_time_ms: Some(5),
                last_check: Some(Utc::now()),
            },
        );

        HealthStatus {
            status: "healthy".to_string(),
            timestamp: Utc::now(),
            services,
            version: Some("1.0.0".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_generation() {
        let spec = generate_openapi_spec();
        assert!(!spec.is_empty());
        assert!(spec.contains("\"openapi\""));
        assert!(spec.contains("\"info\""));
        assert!(spec.contains("\"paths\""));
    }

    #[test]
    fn test_example_data() {
        let schedule_result = examples::example_schedule_result();
        assert_eq!(schedule_result.codebase, "example-package");
        assert!(schedule_result.queue_position > 0);

        let run = examples::example_run();
        assert_eq!(run.campaign, "lintian-fixes");
        assert!(!run.logfilenames.is_empty());

        let proposal = examples::example_merge_proposal();
        assert!(proposal.url.starts_with("https://"));
    }
}
