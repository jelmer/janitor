use chrono::TimeDelta;
use serde::{Deserialize, Serialize};
use sqlx::postgres::types::PgInterval;
use sqlx::{Error, FromRow, PgPool, Row};
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

#[derive(Debug, FromRow)]
pub struct QueueItem {
    pub id: i32,
    pub context: Option<String>,
    pub command: String,
    pub estimated_duration: PgInterval,
    pub campaign: String,
    pub refresh: bool,
    pub requester: Option<String>,
    pub change_set: Option<String>,
    pub codebase: String,
}

impl PartialEq for QueueItem {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for QueueItem {}

impl PartialOrd for QueueItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.id.cmp(&other.id))
    }
}

impl Ord for QueueItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl Hash for QueueItem {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

pub struct Queue<'a> {
    pool: &'a PgPool,
}

#[derive(FromRow)]
pub struct ETA {
    pub position: i64,
    pub wait_time: PgInterval,
}

#[derive(FromRow, Debug, Clone, Serialize, Deserialize, Default)]
pub struct VcsInfo {
    pub branch_url: Option<String>,
    pub subpath: Option<String>,
    pub vcs_type: Option<String>,
}

impl<'a> Queue<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Queue { pool }
    }

    pub async fn get_position(&self, campaign: &str, codebase: &str) -> Result<Option<ETA>, Error> {
        let row: Option<ETA> = sqlx::query_as::<_, ETA>(
            "SELECT position, wait_time FROM queue_positions WHERE codebase = $1 AND suite = $2",
        )
        .bind(codebase)
        .bind(campaign)
        .fetch_optional(self.pool)
        .await?;

        Ok(row)
    }

    pub async fn get_item(&self, queue_id: i32) -> Result<Option<QueueItem>, Error> {
        let row = sqlx::query_as::<_, QueueItem>(
            "SELECT id, context, command, estimated_duration, suite AS campaign, refresh, requester, change_set, codebase
             FROM queue
             WHERE id = $1"
        )
        .bind(queue_id)
        .fetch_optional(self.pool)
        .await?;

        Ok(row)
    }

    /// Get the next item in the queue that is not assigned to any worker
    ///
    /// If `codebase` is provided, only items from that codebase will be considered.
    /// If `campaign` is provided, only items from that campaign will be considered.
    ///
    /// # Arguments
    /// * `codebase` - The codebase to filter by
    /// * `campaign` - The campaign to filter by
    /// * `exclude_hosts` - A set of VCS URL hosts to exclude
    /// * `assigned_queue_items` - A set of queue items that are already assigned
    pub async fn next_item(
        &self,
        codebase: Option<&str>,
        campaign: Option<&str>,
        exclude_hosts: Option<HashSet<String>>,
        assigned_queue_items: Option<HashSet<i32>>,
    ) -> Result<(Option<QueueItem>, Option<VcsInfo>), Error> {
        let mut query = String::from(
            "SELECT
                queue.id, queue.context, queue.command, queue.estimated_duration, queue.suite AS campaign, 
                queue.refresh, queue.requester, queue.change_set, queue.codebase,
                codebase.vcs_type, codebase.branch_url, codebase.subpath
             FROM queue
             LEFT JOIN codebase ON codebase.name = queue.codebase"
        );

        let mut conditions = Vec::new();

        if assigned_queue_items.is_some() {
            conditions.push("NOT (queue.id = ANY($1::int[]))");
        }

        if codebase.is_some() {
            conditions.push("queue.codebase = $2");
        }

        if campaign.is_some() {
            conditions.push("queue.suite = $3");
        }

        if exclude_hosts.is_some() {
            conditions.push(
                "NOT (codebase.branch_url IS NOT NULL AND SUBSTRING(codebase.branch_url from '.*://(?:[^/@]*@)?([^/]*)') = ANY($4::text[]))"
            );
        }

        if !conditions.is_empty() {
            query += " WHERE ";
            query += &conditions.join(" AND ");
        }

        query += " ORDER BY queue.bucket ASC, queue.priority ASC, queue.id ASC LIMIT 1";

        let mut query_builder = sqlx::query(&query);

        if let Some(assigned_queue_items) = assigned_queue_items {
            query_builder =
                query_builder.bind(assigned_queue_items.into_iter().collect::<Vec<_>>());
        }

        if let Some(codebase) = codebase {
            query_builder = query_builder.bind(codebase);
        }

        if let Some(campaign) = campaign {
            query_builder = query_builder.bind(campaign);
        }

        if let Some(exclude_hosts) = exclude_hosts.as_ref() {
            query_builder = query_builder.bind(exclude_hosts.iter().collect::<Vec<_>>());
        }

        let row = query_builder.fetch_optional(self.pool).await?;

        if let Some(row) = row {
            let vcs_info = VcsInfo::from_row(&row)?;

            let queue_item: QueueItem = QueueItem::from_row(&row)?;

            Ok((Some(queue_item), Some(vcs_info)))
        } else {
            Ok((None, None))
        }
    }

    pub async fn add(
        &self,
        codebase: &str,
        command: &str,
        campaign: &str,
        change_set: Option<&str>,
        offset: f64,
        bucket: &str,
        context: Option<&str>,
        estimated_duration: Option<TimeDelta>,
        refresh: bool,
        requester: Option<&str>,
    ) -> Result<(i32, String), Error> {
        let row = sqlx::query(
            "INSERT INTO queue (command, priority, bucket, context, estimated_duration, suite, refresh, requester, change_set, codebase)
             VALUES ($1, (SELECT COALESCE(MIN(priority), 0) FROM queue) + $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (codebase, suite, coalesce(change_set, ''::text))
             DO UPDATE SET context = EXCLUDED.context,
                           priority = EXCLUDED.priority,
                           bucket = EXCLUDED.bucket,
                           estimated_duration = EXCLUDED.estimated_duration,
                           refresh = EXCLUDED.refresh,
                           requester = EXCLUDED.requester,
                           command = EXCLUDED.command,
                           codebase = EXCLUDED.codebase
             WHERE queue.bucket >= EXCLUDED.bucket OR
                   (queue.bucket = EXCLUDED.bucket AND queue.priority >= EXCLUDED.priority)
             RETURNING id, bucket"
        )
        .bind(command)
        .bind(offset)
        .bind(bucket)
        .bind(context)
        .bind(estimated_duration)
        .bind(campaign)
        .bind(refresh)
        .bind(requester)
        .bind(change_set)
        .bind(codebase)
        .fetch_optional(self.pool)
        .await?;

        if let Some(row) = row {
            let id: i32 = row.try_get("id")?;
            let bucket: String = row.try_get("bucket")?;
            Ok((id, bucket))
        } else {
            let row = sqlx::query(
                "SELECT id, bucket FROM queue WHERE codebase = $1 AND suite = $2 AND coalesce(change_set, ''::text) = $3"
            )
            .bind(codebase)
            .bind(campaign)
            .bind(change_set.unwrap_or(""))
            .fetch_one(self.pool)
            .await?;
            let id: i32 = row.try_get("id")?;
            let bucket: String = row.try_get("bucket")?;
            Ok((id, bucket))
        }
    }

    pub async fn get_buckets(&self) -> Result<Vec<(String, i64)>, Error> {
        let rows =
            sqlx::query("SELECT bucket, count(*) FROM queue GROUP BY bucket ORDER BY bucket ASC")
                .fetch_all(self.pool)
                .await?;

        Ok(rows
            .into_iter()
            .map(|row| {
                let bucket: String = row.try_get("bucket").unwrap();
                let count: i64 = row.try_get("count").unwrap();
                (bucket, count)
            })
            .collect())
    }

    /// Iterator for queue items with filtering and limiting capabilities
    ///
    /// This matches the Python iter_queue() method functionality
    ///
    /// # Arguments
    /// * `limit` - Optional limit on number of items to return
    /// * `campaign` - Optional campaign filter
    ///
    /// # Returns
    /// Vector of QueueItem objects in priority order (bucket ASC, priority ASC, id ASC)
    pub async fn iter_queue(
        &self,
        limit: Option<i64>,
        campaign: Option<&str>,
    ) -> Result<Vec<QueueItem>, Error> {
        let mut query = r#"
            SELECT queue.id, queue.context, queue.command, queue.estimated_duration,
                   queue.suite AS campaign, queue.refresh, queue.requester, 
                   queue.change_set, queue.codebase
            FROM queue
        "#
        .to_string();

        let mut conditions = Vec::new();
        let mut bind_count = 0;

        // Add campaign filter if provided
        if campaign.is_some() {
            bind_count += 1;
            conditions.push(format!("queue.suite = ${}", bind_count));
        }

        // Add WHERE clause if we have conditions
        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        // Add ordering (same as next_item)
        query.push_str(" ORDER BY bucket ASC, priority ASC, queue.id ASC");

        // Add limit if provided
        if limit.is_some() {
            bind_count += 1;
            query.push_str(&format!(" LIMIT ${}", bind_count));
        }

        let mut sqlx_query = sqlx::query_as::<_, QueueItem>(&query);

        // Bind parameters in the order they were added
        if let Some(campaign) = campaign {
            sqlx_query = sqlx_query.bind(campaign);
        }
        if let Some(limit) = limit {
            sqlx_query = sqlx_query.bind(limit);
        }

        sqlx_query.fetch_all(self.pool).await
    }

    /// Get queue position with tuple return type matching Python API
    ///
    /// # Returns
    /// (position, wait_time) tuple where both can be None if not found
    pub async fn get_position_tuple(
        &self,
        campaign: &str,
        codebase: &str,
    ) -> Result<(Option<i64>, Option<TimeDelta>), Error> {
        match self.get_position(campaign, codebase).await? {
            Some(eta) => {
                // Convert PgInterval to TimeDelta
                let wait_time = TimeDelta::microseconds(eta.wait_time.microseconds);
                Ok((Some(eta.position), Some(wait_time)))
            }
            None => Ok((None, None)),
        }
    }

    /// Get next queue item with fixed return type matching Python API
    ///
    /// # Returns
    /// (QueueItem, VCS info dict) where VCS dict is never None, just empty
    pub async fn next_item_tuple(
        &self,
        codebase: Option<&str>,
        campaign: Option<&str>,
        exclude_hosts: Option<HashSet<String>>,
        assigned_queue_items: Option<HashSet<i32>>,
    ) -> Result<(Option<QueueItem>, HashMap<String, String>), Error> {
        let (item, vcs_info) = self
            .next_item(codebase, campaign, exclude_hosts, assigned_queue_items)
            .await?;

        // Convert VcsInfo to HashMap, filtering out None values
        let mut vcs_dict = HashMap::new();
        if let Some(vcs) = vcs_info {
            if let Some(branch_url) = vcs.branch_url {
                if !branch_url.is_empty() {
                    vcs_dict.insert("branch_url".to_string(), branch_url);
                }
            }
            if let Some(subpath) = vcs.subpath {
                if !subpath.is_empty() {
                    vcs_dict.insert("subpath".to_string(), subpath);
                }
            }
            if let Some(vcs_type) = vcs.vcs_type {
                if !vcs_type.is_empty() {
                    vcs_dict.insert("vcs_type".to_string(), vcs_type);
                }
            }
        }

        Ok((item, vcs_dict))
    }
}
