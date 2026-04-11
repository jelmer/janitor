//! Pagination utilities for database queries
//!
//! This module provides common pagination patterns to prevent unbounded
//! result sets that could consume excessive memory or cause performance issues.

use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};

/// Standard pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    /// Page number (0-based)
    pub page: u64,
    /// Number of items per page (max 10000)
    pub limit: u64,
    /// Optional cursor for cursor-based pagination
    pub cursor: Option<String>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: 0,
            limit: 1000, // Default 1000 items per page
            cursor: None,
        }
    }
}

impl PaginationParams {
    /// Create pagination params with validated limit
    pub fn new(page: u64, limit: u64) -> Self {
        Self {
            page,
            limit: limit.min(10000), // Cap at 10,000 items
            cursor: None,
        }
    }

    /// Create cursor-based pagination params
    pub fn with_cursor(limit: u64, cursor: Option<String>) -> Self {
        Self {
            page: 0,
            limit: limit.min(10000),
            cursor,
        }
    }

    /// Calculate SQL OFFSET value
    pub fn offset(&self) -> u64 {
        self.page * self.limit
    }

    /// Get SQL LIMIT value
    pub fn sql_limit(&self) -> i64 {
        self.limit as i64
    }

    /// Get SQL OFFSET value
    pub fn sql_offset(&self) -> i64 {
        self.offset() as i64
    }
}

/// Paginated response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// The actual data items
    pub data: Vec<T>,
    /// Pagination metadata
    pub pagination: PaginationMetadata,
}

/// Metadata about pagination state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMetadata {
    /// Current page (0-based)
    pub page: u64,
    /// Items per page
    pub limit: u64,
    /// Total count (if available)
    pub total_count: Option<u64>,
    /// Whether there are more pages
    pub has_next: bool,
    /// Next cursor (for cursor-based pagination)
    pub next_cursor: Option<String>,
}

/// Execute a paginated query with count (simplified version without complex binding)
pub async fn execute_paginated_query<T>(
    pool: &PgPool,
    base_query: &str,
    count_query: &str,
    params: &PaginationParams,
) -> Result<PaginatedResponse<T>, sqlx::Error>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    // Build paginated query
    let paginated_query = format!(
        "{} LIMIT {} OFFSET {}",
        base_query,
        params.sql_limit(),
        params.sql_offset()
    );

    // Execute queries concurrently
    let (data_result, count_result) = tokio::try_join!(
        sqlx::query_as::<_, T>(&paginated_query).fetch_all(pool),
        execute_count_query_simple(pool, count_query)
    )?;

    let has_next = data_result.len() as u64 == params.limit;
    let total_count = Some(count_result);

    Ok(PaginatedResponse {
        data: data_result,
        pagination: PaginationMetadata {
            page: params.page,
            limit: params.limit,
            total_count,
            has_next,
            next_cursor: None, // TODO: Implement cursor-based pagination
        },
    })
}

/// Execute count query simplified
async fn execute_count_query_simple(pool: &PgPool, query: &str) -> Result<u64, sqlx::Error> {
    let row = sqlx::query(query).fetch_one(pool).await?;
    let count: i64 = row.try_get(0)?;
    Ok(count as u64)
}

/// Simplified pagination for queries without complex binding
pub async fn paginate_simple_query<T>(
    pool: &PgPool,
    base_query: &str,
    params: &PaginationParams,
) -> Result<PaginatedResponse<T>, sqlx::Error>
where
    T: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
{
    let paginated_query = format!(
        "{} LIMIT {} OFFSET {}",
        base_query,
        params.sql_limit(),
        params.sql_offset()
    );

    let data = sqlx::query_as::<_, T>(&paginated_query)
        .fetch_all(pool)
        .await?;

    let has_next = data.len() as u64 == params.limit;

    Ok(PaginatedResponse {
        data,
        pagination: PaginationMetadata {
            page: params.page,
            limit: params.limit,
            total_count: None, // Don't compute total for simple queries
            has_next,
            next_cursor: None,
        },
    })
}

/// Cursor-based pagination utilities for large datasets
pub struct CursorPagination {
    /// The cursor field name (e.g., "id", "created_at")
    pub cursor_field: String,
    /// The sort order ("ASC" or "DESC")
    pub sort_order: String,
}

impl CursorPagination {
    pub fn new(cursor_field: String, sort_order: String) -> Self {
        Self {
            cursor_field,
            sort_order,
        }
    }

    /// Build WHERE clause for cursor-based pagination
    pub fn build_cursor_where(&self, cursor_value: Option<&str>) -> String {
        match cursor_value {
            Some(cursor) => {
                let operator = if self.sort_order == "ASC" { ">" } else { "<" };
                format!("WHERE {} {} '{}'", self.cursor_field, operator, cursor)
            }
            None => String::new(),
        }
    }

    /// Build ORDER BY clause
    pub fn build_order_by(&self) -> String {
        format!("ORDER BY {} {}", self.cursor_field, self.sort_order)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_params_default() {
        let params = PaginationParams::default();
        assert_eq!(params.page, 0);
        assert_eq!(params.limit, 1000);
        assert_eq!(params.offset(), 0);
    }

    #[test]
    fn test_pagination_params_offset() {
        let params = PaginationParams::new(2, 100);
        assert_eq!(params.page, 2);
        assert_eq!(params.limit, 100);
        assert_eq!(params.offset(), 200);
    }

    #[test]
    fn test_pagination_params_limit_cap() {
        let params = PaginationParams::new(0, 20000);
        assert_eq!(params.limit, 10000); // Should be capped at 10000
    }

    #[test]
    fn test_cursor_pagination_where_clause() {
        let cursor = CursorPagination::new("id".to_string(), "ASC".to_string());

        let where_clause = cursor.build_cursor_where(Some("123"));
        assert_eq!(where_clause, "WHERE id > '123'");

        let where_clause = cursor.build_cursor_where(None);
        assert_eq!(where_clause, "");
    }

    #[test]
    fn test_cursor_pagination_desc() {
        let cursor = CursorPagination::new("created_at".to_string(), "DESC".to_string());

        let where_clause = cursor.build_cursor_where(Some("2023-01-01"));
        assert_eq!(where_clause, "WHERE created_at < '2023-01-01'");

        let order_clause = cursor.build_order_by();
        assert_eq!(order_clause, "ORDER BY created_at DESC");
    }
}
