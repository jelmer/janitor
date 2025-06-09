/// Core janitor database schema
pub const CORE_SCHEMA: &str = include_str!("../py/janitor/state.sql");

/// Debian-specific database schema extensions
pub const DEBIAN_SCHEMA: &str = include_str!("../py/janitor/debian/debian.sql");

#[cfg(feature = "testing")]
/// Set up a test database with core janitor schema
pub async fn setup_test_database(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    // Execute the entire schema as one statement - PostgreSQL can handle this
    sqlx::query(CORE_SCHEMA).execute(pool).await?;
    Ok(())
}

#[cfg(feature = "testing")]
/// Set up a test database with Debian extensions
pub async fn setup_debian_test_database(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    setup_test_database(pool).await?;
    sqlx::query(DEBIAN_SCHEMA).execute(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_schema_is_not_empty() {
        assert!(!CORE_SCHEMA.is_empty());
        assert!(CORE_SCHEMA.len() > 1000); // Should be a substantial schema
    }

    #[test]
    fn test_debian_schema_is_not_empty() {
        assert!(!DEBIAN_SCHEMA.is_empty());
        assert!(DEBIAN_SCHEMA.len() > 100); // Debian schema is smaller
    }

    #[test]
    fn test_core_schema_contains_expected_tables() {
        // Check for key tables
        assert!(CORE_SCHEMA.contains("CREATE TABLE IF NOT EXISTS codebase"));
        assert!(CORE_SCHEMA.contains("CREATE TABLE IF NOT EXISTS merge_proposal"));
        assert!(CORE_SCHEMA.contains("CREATE TABLE IF NOT EXISTS run"));
        assert!(CORE_SCHEMA.contains("CREATE TABLE IF NOT EXISTS change_set"));
        assert!(CORE_SCHEMA.contains("CREATE TABLE IF NOT EXISTS queue"));
        assert!(CORE_SCHEMA.contains("CREATE TABLE IF NOT EXISTS named_publish_policy"));
        assert!(CORE_SCHEMA.contains("CREATE TABLE IF NOT EXISTS worker"));
    }

    #[test]
    fn test_debian_schema_contains_expected_table() {
        assert!(DEBIAN_SCHEMA.contains("CREATE TABLE debian_build"));
        assert!(DEBIAN_SCHEMA.contains("run_id"));
        assert!(DEBIAN_SCHEMA.contains("distribution"));
        assert!(DEBIAN_SCHEMA.contains("version debversion"));
        assert!(DEBIAN_SCHEMA.contains("source text"));
    }

    #[test]
    fn test_core_schema_contains_sql_elements() {
        // Check for various SQL elements
        assert!(CORE_SCHEMA.contains("CREATE OR REPLACE VIEW"));
        assert!(
            CORE_SCHEMA.contains("CREATE INDEX") || CORE_SCHEMA.contains("CREATE UNIQUE INDEX")
        );
        assert!(CORE_SCHEMA.contains("CREATE OR REPLACE FUNCTION"));
        assert!(CORE_SCHEMA.contains("references"));
        assert!(CORE_SCHEMA.contains("not null"));
    }

    #[test]
    fn test_schemas_are_different() {
        assert_ne!(CORE_SCHEMA, DEBIAN_SCHEMA);
        assert!(CORE_SCHEMA.len() > DEBIAN_SCHEMA.len());
    }
}
