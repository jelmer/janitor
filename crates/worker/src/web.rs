use askama_axum::Template;
use janitor::api::worker::{Assignment, Metadata};

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate<'a> {
    pub assignment: Option<&'a Assignment>,
    pub metadata: Option<&'a Metadata>,
    pub lognames: Option<Vec<&'a str>>,
}

#[derive(Template)]
#[template(path = "artifact_index.html")]
pub struct ArtifactIndexTemplate<'a> {
    pub names: Vec<&'a str>,
}

#[derive(Template)]
#[template(path = "log_index.html")]
pub struct LogIndexTemplate<'a> {
    pub names: Vec<&'a str>,
}

