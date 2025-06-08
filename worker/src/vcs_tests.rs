#[cfg(test)]
mod tests {
    use super::*;
    use janitor::vcs::VcsType;

    #[test]
    fn test_result_to_changes_nothing_to_do() {
        let result = silver_platter::debian::codemod::CommandResult::Nothing {
            reason: "No changes needed".to_string(),
        };
        
        let changes = result_to_changes(&result);
        assert_eq!(changes.branches, vec![]);
        assert_eq!(changes.tags, vec![]);
        assert_eq!(changes.description, Some("No changes needed".to_string()));
    }

    #[test]
    fn test_result_to_changes_with_branches() {
        use breezyshim::RevisionId;
        
        let branches = vec![
            ("main".to_string(), 
             "main-branch".to_string(), 
             Some(RevisionId::from("base-rev".as_bytes())), 
             Some(RevisionId::from("new-rev".as_bytes()))),
        ];
        
        let tags = vec![
            ("v1.0".to_string(), Some(RevisionId::from("tag-rev".as_bytes()))),
        ];
        
        let result = silver_platter::debian::codemod::CommandResult::Success { 
            changes_made: "Updated dependencies".to_string(),
            branches,
            tags,
            ..Default::default()
        };
        
        let changes = result_to_changes(&result);
        assert_eq!(changes.branches.len(), 1);
        assert_eq!(changes.tags.len(), 1);
        assert_eq!(changes.description, Some("Updated dependencies".to_string()));
    }

    #[test]
    fn test_result_to_changes_repositories() {
        let repositories = vec![
            ("https://example.com/repo1".to_string(), "main".to_string()),
            ("https://example.com/repo2".to_string(), "develop".to_string()),
        ];
        
        let result = silver_platter::debian::codemod::CommandResult::Success {
            repositories: Some(repositories.clone()),
            ..Default::default()
        };
        
        let changes = result_to_changes(&result);
        assert_eq!(changes.repositories, repositories);
    }

    #[test]
    fn test_ensure_dir_exists_new() {
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let new_path = temp_dir.path().join("subdir").join("nested");
        
        ensure_dir_exists(&new_path).unwrap();
        assert!(new_path.exists());
        assert!(new_path.is_dir());
    }

    #[test]
    fn test_ensure_dir_exists_existing() {
        use tempfile::TempDir;
        
        let temp_dir = TempDir::new().unwrap();
        let existing_path = temp_dir.path();
        
        // Should not error on existing directory
        ensure_dir_exists(existing_path).unwrap();
        assert!(existing_path.exists());
    }

    #[test]
    fn test_default_vcs_type_from_path_bzr() {
        let path = std::path::Path::new("/path/to/repo/.bzr");
        let vcs_type = default_vcs_type_from_path(Some(path));
        assert_eq!(vcs_type, Some(VcsType::Bzr));
    }

    #[test]
    fn test_default_vcs_type_from_path_git() {
        let path = std::path::Path::new("/path/to/repo/.git");
        let vcs_type = default_vcs_type_from_path(Some(path));
        assert_eq!(vcs_type, Some(VcsType::Git));
    }

    #[test]
    fn test_default_vcs_type_from_path_none() {
        let path = std::path::Path::new("/path/to/repo");
        let vcs_type = default_vcs_type_from_path(Some(path));
        assert_eq!(vcs_type, None);
    }

    #[test]
    fn test_default_vcs_type_from_path_empty() {
        let vcs_type = default_vcs_type_from_path(None);
        assert_eq!(vcs_type, None);
    }

    // Note: Testing push_branch, import_branches_bzr, and import_branches functions
    // would require mock implementations of breezyshim types or integration tests
    // with actual VCS repositories
}