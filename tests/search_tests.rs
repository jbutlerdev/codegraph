//! Search tests

use codegraph::db::{Database, Knowledge, FileInput, SearchParams};
use tempfile::tempdir;

#[test]
fn test_search_by_path() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    let knowledge = Knowledge::new_local(
        "search-test-id",
        "test/repo",
        "/path/to/repo",
    );
    db.insert_knowledge(&knowledge).unwrap();
    
    // Add files
    for path in &["src/auth/login.rs", "src/auth/session.rs", "src/main.rs"] {
        let file_input = FileInput {
            knowledge_id: "search-test-id".to_string(),
            relative_path: path.to_string(),
            language: Some("rust".to_string()),
            sha256: format!("sha-{}", path),
            size_bytes: 100,
            purpose: Some(format!("Purpose for {}", path)),
            summary: Some(format!("Summary for {}", path)),
            business_context: None,
        };
        db.upsert_file(&file_input).unwrap();
    }
    
    // Search by path
    let params = SearchParams {
        query: "auth".to_string(),
        knowledge_id: Some("search-test-id".to_string()),
        path_prefix: None,
        exclude_suffixes: vec![],
        exclude_contains: vec![],
        limit: 10,
    };
    
    let results = db.search_paths(&params).unwrap();
    assert_eq!(results.len(), 2); // login.rs and session.rs
    
    // Verify paths
    let paths: Vec<_> = results.iter().map(|r| r.path.as_str()).collect();
    assert!(paths.contains(&"src/auth/login.rs"));
    assert!(paths.contains(&"src/auth/session.rs"));
    assert!(!paths.contains(&"src/main.rs"));
}

#[test]
fn test_search_by_purpose() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    let knowledge = Knowledge::new_local(
        "purpose-test-id",
        "test/repo",
        "/path/to/repo",
    );
    db.insert_knowledge(&knowledge).unwrap();
    
    // Add files with different purposes
    let files = vec![
        ("src/auth.rs", "Handles user authentication and login", "Authentication module"),
        ("src/main.rs", "Main entry point of the application", "Application startup"),
        ("src/logger.rs", "Handles logging throughout the application", "Logging functionality"),
    ];
    
    for (path, purpose, _) in &files {
        let file_input = FileInput {
            knowledge_id: "purpose-test-id".to_string(),
            relative_path: path.to_string(),
            language: Some("rust".to_string()),
            sha256: format!("sha-{}", path),
            size_bytes: 100,
            purpose: Some(purpose.to_string()),
            summary: None,
            business_context: None,
        };
        db.upsert_file(&file_input).unwrap();
    }
    
    // Search for "auth" in purpose
    let params = SearchParams {
        query: "authentication".to_string(),
        knowledge_id: Some("purpose-test-id".to_string()),
        path_prefix: None,
        exclude_suffixes: vec![],
        exclude_contains: vec![],
        limit: 10,
    };
    
    let results = db.search_purpose(&params).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].path, "src/auth.rs");
}

#[test]
fn test_search_with_limit() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    let knowledge = Knowledge::new_local(
        "limit-test-id",
        "test/repo",
        "/path/to/repo",
    );
    db.insert_knowledge(&knowledge).unwrap();
    
    // Add many files
    for i in 0..20 {
        let file_input = FileInput {
            knowledge_id: "limit-test-id".to_string(),
            relative_path: format!("src/file{}.rs", i),
            language: Some("rust".to_string()),
            sha256: format!("sha-{}", i),
            size_bytes: 100,
            purpose: Some("Test file purpose".to_string()),
            summary: None,
            business_context: None,
        };
        db.upsert_file(&file_input).unwrap();
    }
    
    let params = SearchParams {
        query: "test".to_string(),
        knowledge_id: Some("limit-test-id".to_string()),
        path_prefix: None,
        exclude_suffixes: vec![],
        exclude_contains: vec![],
        limit: 5,
    };
    
    let results = db.search_paths(&params).unwrap();
    assert!(results.len() <= 5);
}

#[test]
fn test_search_knowledge_isolation() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    // Create two separate knowledge bases
    for knowledge_id in &["repo1", "repo2"] {
        let knowledge = Knowledge::new_local(
            knowledge_id,
            *knowledge_id,
            &format!("/path/to/{}", knowledge_id),
        );
        db.insert_knowledge(&knowledge).unwrap();
        
        let file_input = FileInput {
            knowledge_id: knowledge_id.to_string(),
            relative_path: "auth.rs".to_string(),
            language: Some("rust".to_string()),
            sha256: format!("sha-{}", knowledge_id),
            size_bytes: 100,
            purpose: Some("Auth for repo".to_string()),
            summary: None,
            business_context: None,
        };
        db.upsert_file(&file_input).unwrap();
    }
    
    // Search only in repo1
    let params = SearchParams {
        query: "auth".to_string(),
        knowledge_id: Some("repo1".to_string()),
        path_prefix: None,
        exclude_suffixes: vec![],
        exclude_contains: vec![],
        limit: 10,
    };
    
    let results = db.search_paths(&params).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].knowledge_id, "repo1");
}

#[test]
fn test_search_without_knowledge_filter() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    // Create two separate knowledge bases
    for knowledge_id in &["repo1", "repo2"] {
        let knowledge = Knowledge::new_local(
            knowledge_id,
            *knowledge_id,
            &format!("/path/to/{}", knowledge_id),
        );
        db.insert_knowledge(&knowledge).unwrap();
        
        let file_input = FileInput {
            knowledge_id: knowledge_id.to_string(),
            relative_path: "shared.rs".to_string(),
            language: Some("rust".to_string()),
            sha256: format!("sha-{}", knowledge_id),
            size_bytes: 100,
            purpose: Some("Shared functionality".to_string()),
            summary: None,
            business_context: None,
        };
        db.upsert_file(&file_input).unwrap();
    }
    
    // Search across all repos
    let params = SearchParams {
        query: "shared".to_string(),
        knowledge_id: None, // No filter = search all
        path_prefix: None,
        exclude_suffixes: vec![],
        exclude_contains: vec![],
        limit: 10,
    };
    
    let results = db.search_paths(&params).unwrap();
    assert_eq!(results.len(), 2); // Should find in both repos
}
