"""Tests for database schema export functionality."""

from janitor import get_core_schema, get_debian_schema


def test_get_core_schema():
    """Test that get_core_schema returns valid SQL."""
    schema = get_core_schema()

    # Check that we got a non-empty string
    assert isinstance(schema, str)
    assert len(schema) > 0

    # Check for key tables in the core schema
    assert "CREATE TABLE IF NOT EXISTS codebase" in schema
    assert "CREATE TABLE IF NOT EXISTS merge_proposal" in schema
    assert "CREATE TABLE IF NOT EXISTS run" in schema
    assert "CREATE TABLE IF NOT EXISTS change_set" in schema
    assert "CREATE TABLE IF NOT EXISTS queue" in schema
    assert "CREATE TABLE IF NOT EXISTS named_publish_policy" in schema
    assert "CREATE TABLE IF NOT EXISTS worker" in schema

    # Check for views
    assert "CREATE OR REPLACE VIEW" in schema

    # Check for functions
    assert "CREATE OR REPLACE FUNCTION" in schema


def test_get_debian_schema():
    """Test that get_debian_schema returns valid SQL."""
    schema = get_debian_schema()

    # Check that we got a non-empty string
    assert isinstance(schema, str)
    assert len(schema) > 0

    # Check for the debian_build table
    assert "CREATE TABLE debian_build" in schema

    # Check for expected columns
    assert "run_id" in schema
    assert "distribution" in schema
    assert "version debversion" in schema
    assert "source text" in schema
    assert "binary_packages" in schema


def test_schemas_are_different():
    """Test that core and debian schemas are different."""
    core = get_core_schema()
    debian = get_debian_schema()

    assert core != debian
    assert len(core) > len(debian)  # Core schema should be much larger


def test_core_schema_sql_syntax():
    """Test that core schema contains valid SQL syntax markers."""
    schema = get_core_schema()

    # Check for common SQL keywords
    sql_keywords = [
        "CREATE",
        "TABLE",
        "INDEX",
        "VIEW",
        "FUNCTION",
        "references",
        "not null",
    ]

    for keyword in sql_keywords:
        assert keyword in schema, (
            f"Expected SQL keyword '{keyword}' not found in schema"
        )


def test_debian_schema_sql_syntax():
    """Test that debian schema contains valid SQL syntax markers."""
    schema = get_debian_schema()

    # Check for basic SQL structure
    assert "CREATE TABLE" in schema
    assert "CREATE INDEX" in schema
    assert "not null" in schema
    assert ");" in schema  # End of table definition
    assert "references run (id)" in schema  # Foreign key reference
