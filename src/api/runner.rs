/// Sent when the publish-status for a run changes.
#[derive(Debug, serde::Deserialize, serde::Serialize, Clone)]
struct PublishStatusPubsub {
    /// The codebase.
    codebase: String,

    /// The run ID.
    run_id: crate::RunId,

    /// The new publish-status.
    #[serde(rename = "publish-status")]
    publish_status: crate::api::RunPublishStatus,
}
