use crate::queue::Item as QueueItem;
use async_trait::async_trait;
use janitor::api::worker::LintianConfig;
use janitor::config::{Campaign, Distribution};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::path::Path;

#[async_trait]
pub trait BuilderResult: Serialize + Deserialize<'static> {
    fn from_directory(path: &Path) -> Result<Self, Error>;

    async fn store(&self, conn: &PgPool, run_id: &str) -> Result<(), sqlx::Error>;

    fn artifact_filenames(&self) -> Vec<String>;
}

#[async_trait]
pub trait Builder {
    async fn config(
        &self,
        conn: &PgPool,
        campaign_config: &Campaign,
        queue_item: &QueueItem,
    ) -> Result<serde_json::Value, Error>;

    async fn build_env(
        &self,
        conn: &PgPool,
        campaign_config: &Campaign,
        queue_item: &QueueItem,
    ) -> Result<HashMap<String, String>, Error>;

    fn additional_colocated_branches(&self, main_branch: &str) -> Vec<String>;
}

pub struct GenericResult;

impl BuilderResult for GenericResult {
    fn from_directory(path: &Path) -> Result<Self, Error> {
        Ok(Self)
    }

    async fn store(&self, conn: &PgPool, run_id: &str) -> Result<(), sqlx::Error> {
        Ok(())
    }

    fn artifact_filenames(&self) -> Vec<String> {
        Vec::new()
    }
}

pub struct GenericBuilder {
    dep_server_url: url::Url,
}

impl Builder for GenericBuilder {
    async fn config(
        &self,
        conn: &PgPool,
        campaign_config: &Campaign,
        queue_item: &QueueItem,
    ) -> Result<serde_json::Value, Error> {
        let mut config = janitor::api::worker::GenericBuildConfig::default();
        if let Some(chroot) = &campaign_config.generic_build.chroot {
            config.chroot = Some(chroot.to_string());
        }
        config.dep_server_url = Some(self.dep_server_url);
        Ok(serde_json::to_value(config))
    }

    async fn build_env(
        &self,
        conn: &PgPool,
        campaign_config: &Campaign,
        queue_item: &QueueItem,
    ) -> Result<HashMap<String, String>, Error> {
        Ok(HashMap::new())
    }

    fn additional_colocated_branches(&self, main_branch: &str) -> Vec<String> {
        Vec::new()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DebianResult {
    source: String,
    build_version: String,
    build_distribution: String,
    changes_filenames: Vec<String>,
    lintian_result: Option<String>,
    binary_packages: Vec<String>,
}

impl BuilderResult for DebianResult {
    fn from_directory(path: &Path) -> Result<Self, Error> {
        let (changes_filenames, source, build_version, build_distribution, binary_packages) =
            match find_changes(path) {
                Ok((
                    changes_filenames,
                    source,
                    build_version,
                    build_distribution,
                    binary_packages,
                )) => {
                    log::info!(
                    "Found changes files {:?}, source {}, build version {}, distribution: {}, binary packages: {:?}",
                    source,
                    changes_filenames,
                    build_version,
                    build_distribution,
                    binary_packages,
                );

                    (
                        changes_filenames,
                        source,
                        build_version,
                        build_distribution,
                        binary_packages,
                    )
                }
                Err(e) => {
                    log::info!("No changes file found: {}", e);
                    return Ok(Self {
                        source: "".to_string(),
                        build_version: "".to_string(),
                        build_distribution: "".to_string(),
                        changes_filenames: Vec::new(),
                        lintian_result: None,
                        binary_packages: Vec::new(),
                    });
                }
            };
        Ok(Self {
            source,
            build_version,
            build_distribution,
            changes_filenames,
            binary_packages,
            lintian_result: None,
        })
    }

    async fn store(&self, conn: &PgPool, run_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query("INSERT INTO debian_build (run_id, source, version, distribution, lintian_result, binary_packages) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(run_id)
            .bind(&self.source)
            .bind(&self.build_version)
            .bind(&self.build_distribution)
            .bind(&self.lintian_result)
            .bind(&self.binary_packages)
            .execute(conn)
            .await?;
        Ok(())
    }

    fn artifact_filenames(&self) -> Vec<String> {
        let mut ret = Vec::new();
        for changes_filename in &self.changes_filenames {
            let changes_path = self.output_directory.join(changes_filename);
            ret.extend(crate::changes_filenames(&changes_path));
            ret.push(changes_filename);
        }
        ret
    }
}

pub struct DebianBuilder {
    distro_config: Distribution,
    apt_location: Option<String>,
    dep_server_url: Option<url::Url>,
}

impl DebianBuilder {
    pub fn new(
        distro_config: Distribution,
        apt_location: Option<String>,
        dep_server_url: Option<url::Url>,
    ) -> Self {
        Self {
            distro_config,
            apt_location,
            dep_server_url,
        }
    }
}

impl Builder for DebianBuilder {
    async fn config(
        &self,
        conn: &PgPool,
        campaign_config: &Campaign,
        queue_item: &QueueItem,
    ) -> Result<serde_json::Value, Error> {
        let mut config = janitor::api::worker::DebianBuildConfig::default();
        config.lintian = serde_json::to_string(&LintianConfig {
            profile: self.distro_config.lintian_profile.clone(),
            suppress_tags: Some(self.distro_config.lintian_suppress_tag.clone()),
        })?;

        let mut extra_janitor_distributions = Vec::new();
        extra_janitor_distributions.extend(
            campaign_config
                .debian_build()
                .extra_build_distribution
                .iter()
                .cloned(),
        );
        if let Some(change_set) = &queue_item.change_set {
            extra_janitor_distributions.push(format!("cs/{}", change_set));
        }

        // TODO(jelmer): Ship build-extra-repositories-keys, and specify [signed-by] here
        let build_extra_repositories = extra_janitor_distributions
            .iter()
            .map(|suite| {
                format!(
                    "deb [trusted=yes] {} {} main",
                    self.apt_location.as_ref().unwrap(),
                    suite
                )
            })
            .collect::<Vec<_>>();
        config.extra_repositories = Some(build_extra_repositories);

        let build_distribution = campaign_config
            .debian_build()
            .build_distribution
            .as_deref()
            .unwrap_or(&campaign_config.name());
        config.build_distribution = build_distribution;

        let build_suffix = campaign_config
            .debian_build()
            .build_suffix
            .as_deref()
            .unwrap_or("");
        config.build_suffix = Some(build_suffix.to_string());

        config.build_command =
            if let Some(build_command) = &campaign_config.debian_build().build_command {
                Some(build_command.to_string())
            } else if let Some(build_command) = &self.distro_config.build_command {
                Some(build_command.to_string())
            } else {
                None
            };

        let last_build_version: (debversion::Version, ) = sqlx::query("SELECT MAX(debian_build.version) FROM run LEFT JOIN debian_build ON debian_build.run_id = run.id WHERE debian_build.version IS NOT NULL AND run.codebase = $1 AND debian_build.distribution = $2")
            .bind(&queue_item.codebase)
            .bind(&config.build_distribution)
            .fetch_one(conn)
            .await?;

        if let Some(last_build_version) = last_build_version {
            config.last_build_version = last_build_version;
        }

        config.chroot = if let Some(chroot) = &campaign_config.debian_build().chroot {
            Some(chroot.to_string())
        } else if let Some(chroot) = &self.distro_config.chroot {
            Some(chroot.to_string())
        } else {
            None
        };

        if let (Some(archive_mirror_uri), Some(component)) = (
            &self.distro_config.archive_mirror_uri,
            &self.distro_config.component,
        ) {
            config.apt_repository = Some(format!(
                "{} {} {}",
                archive_mirror_uri,
                self.distro_config.name,
                component.join(" ")
            ));
            config.apt_repository_signed_by = self.distro_config.signed_by.clone();
        }

        config.dep_server_url = Some(self.dep_server_url.to_string());

        config
    }

    async fn build_env(
        &self,
        conn: &PgPool,
        campaign_config: &Campaign,
        queue_item: &QueueItem,
    ) -> Result<HashMap<String, String>, Error> {
        let mut env = HashMap::new();
        if let Some(distro_name) = self.distro_config.name {
            env.insert("DISTRIBUTION", distro_name);
        }

        env.insert(
            "DEB_VENDOR",
            self.distro_config.vendor.and_then(|| crate::dpkg_vendor()),
        );

        if let Some(chroot) = &campaign_config.debian_build().chroot {
            env.insert("CHROOT", chroot.to_string());
        } else if let Some(chroot) = &self.distro_config.chroot {
            env.insert("CHROOT", chroot.to_string());
        }

        if let (Some(archive_mirror_uri), Some(component)) = (
            &self.distro_config.archive_mirror_uri,
            &self.distro_config.component,
        ) {
            env.insert(
                "APT_REPOSITORY",
                format!(
                    "{} {} {}",
                    archive_mirror_uri,
                    self.distro_config.name,
                    component.join(" ")
                ),
            );
        }

        // TODO(jelmer): Set env["APT_REPOSITORY_KEY"]

        env
    }

    fn additional_colocated_branches(&self, main_branch: &str) -> Vec<String> {
        pick_additional_colocated_branches(main_branch)
    }
}

/*
pub fn get_builder(config, campaign_config, apt_archive_url=None, dep_server_url=None):
    if campaign_config.HasField("debian_build"):
        try:
            distribution = get_distribution(
                config, campaign_config.debian_build.base_distribution
            )
        except KeyError as e:
            raise NotImplementedError(
                "Unsupported distribution: "
                f"{campaign_config.debian_build.base_distribution}"
            ) from e
        return DebianBuilder(
            distribution,
            apt_archive_url,
            dep_server_url,
        )
    elif campaign_config.HasField("generic_build"):
        return GenericBuilder(dep_server_url)
    else:
        raise NotImplementedError("no supported build type")


*/
