//! Integration tests for the full indexing pipeline

use codegraph::db::{Database, Knowledge, FileInput};
use std::fs;
use tempfile::tempdir;

/// Test basic ingestion flow: create knowledge, add files, verify state transitions
#[test]
fn test_knowledge_state_transitions() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    // Create knowledge in CREATED state
    let knowledge = Knowledge::new_local("test-repo", "test/repo", "/path/to/repo");
    db.insert_knowledge(&knowledge).unwrap();
    
    let retrieved = db.get_knowledge("test-repo").unwrap().unwrap();
    assert_eq!(retrieved.state, codegraph::db::KnowledgeState::Created);
    
    // Transition to QUEUED
    db.update_knowledge_state("test-repo", codegraph::db::KnowledgeState::Queued).unwrap();
    let retrieved = db.get_knowledge("test-repo").unwrap().unwrap();
    assert_eq!(retrieved.state, codegraph::db::KnowledgeState::Queued);
    
    // Transition to INGESTED (files being added)
    db.update_knowledge_state("test-repo", codegraph::db::KnowledgeState::Ingested).unwrap();
    let retrieved = db.get_knowledge("test-repo").unwrap().unwrap();
    assert_eq!(retrieved.state, codegraph::db::KnowledgeState::Ingested);
    
    // Transition to PROCESSING
    db.update_knowledge_state("test-repo", codegraph::db::KnowledgeState::Processing).unwrap();
    let retrieved = db.get_knowledge("test-repo").unwrap().unwrap();
    assert_eq!(retrieved.state, codegraph::db::KnowledgeState::Processing);
    
    // Transition to PROCESSED
    db.update_knowledge_state("test-repo", codegraph::db::KnowledgeState::Processed).unwrap();
    let retrieved = db.get_knowledge("test-repo").unwrap().unwrap();
    assert_eq!(retrieved.state, codegraph::db::KnowledgeState::Processed);
}

/// Test full file analysis workflow
#[test]
fn test_file_analysis_workflow() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    let knowledge = Knowledge::new_local("analysis-test", "test/repo", "/path/to/repo");
    db.insert_knowledge(&knowledge).unwrap();
    
    // Simulate file analysis (like what LLM produces)
    let file_input = FileInput {
        knowledge_id: "analysis-test".to_string(),
        relative_path: "src/auth/login.rs".to_string(),
        language: Some("typescript".to_string()),
        sha256: "abc123def456".to_string(),
        size_bytes: 1523,
        purpose: Some("Handles user authentication and login flow".to_string()),
        summary: Some("This module manages user authentication including password verification, session creation, and token generation. It integrates with the auth provider and provides hooks for MFA.".to_string()),
        business_context: Some("Critical for security - handles all user credentials. Any bug here could expose user accounts.".to_string()),
    };
    
    let file_id = db.upsert_file(&file_input).unwrap();
    
    // Create entities
    let kw1 = db.get_or_create_keyword("authentication").unwrap();
    let kw2 = db.get_or_create_keyword("password").unwrap();
    let kw3 = db.get_or_create_keyword("session").unwrap();
    
    let class_id = db.get_or_create_class("AuthService (~L3-58): handles user login and session management").unwrap();
    let fn_id = db.get_or_create_function("handle_login (~L60-100): authenticates user credentials").unwrap();
    let fn_id2 = db.get_or_create_function("create_session (~L120-150): creates secure session token").unwrap();
    
    let module_int = db.get_or_create_module("./session", false).unwrap();
    let module_ext = db.get_or_create_module("bcrypt", true).unwrap();
    
    // Link entities to file
    db.link_file_keywords(file_id, &[kw1, kw2, kw3]).unwrap();
    db.link_file_classes(file_id, &[class_id]).unwrap();
    db.link_file_functions(file_id, &[fn_id, fn_id2]).unwrap();
    db.link_file_imports_internal(file_id, &[module_int]).unwrap();
    db.link_file_imports_external(file_id, &[module_ext]).unwrap();
    
    // Verify all links
    let keywords = db.get_file_keywords(file_id).unwrap();
    assert_eq!(keywords.len(), 3);
    assert!(keywords.contains(&"authentication".to_string()));
    
    let classes = db.get_file_classes(file_id).unwrap();
    assert_eq!(classes.len(), 1);
    assert!(classes[0].contains("AuthService"));
    
    let functions = db.get_file_functions(file_id).unwrap();
    assert_eq!(functions.len(), 2);
    
    let (internal, external) = db.get_file_imports(file_id).unwrap();
    assert_eq!(internal, vec!["./session"]);
    assert_eq!(external, vec!["bcrypt"]);
}

/// Test search across all channels for a realistic analysis
#[test]
fn test_multi_channel_search() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    let knowledge = Knowledge::new_local("search-test", "test/repo", "/path/to/repo");
    db.insert_knowledge(&knowledge).unwrap();
    
    // Add multiple files with different characteristics
    let files = vec![
        ("src/auth/login.rs", "Handles user authentication", "Authentication module for login", "security"),
        ("src/auth/session.rs", "Manages user sessions", "Session management for web app", "cookies"),
        ("src/api/users.rs", "User API endpoints", "REST API for user operations", "rest"),
        ("src/utils/logger.rs", "Logging utility", "Application logging", "debugging"),
    ];
    
    for (path, purpose, summary, keyword) in &files {
        let file_input = FileInput {
            knowledge_id: "search-test".to_string(),
            relative_path: path.to_string(),
            language: Some("typescript".to_string()),
            sha256: format!("sha-{}", path),
            size_bytes: 100,
            purpose: Some(purpose.to_string()),
            summary: Some(summary.to_string()),
            business_context: None,
        };
        let file_id = db.upsert_file(&file_input).unwrap();
        let kw_id = db.get_or_create_keyword(keyword).unwrap();
        db.link_file_keywords(file_id, &[kw_id]).unwrap();
    }
    
    // Search for "auth" should find auth/login.rs
    let params = codegraph::db::SearchParams {
        query: "auth".to_string(),
        knowledge_id: Some("search-test".to_string()),
        path_prefix: None,
        exclude_suffixes: vec![],
        exclude_contains: vec![],
        limit: 10,
    };
    
    let purpose_results = db.search_purpose(&params).unwrap();
    assert!(!purpose_results.is_empty());
    assert!(purpose_results.iter().any(|r| r.path.contains("auth")));
    
    // Search for "session" 
    let params2 = codegraph::db::SearchParams {
        query: "session".to_string(),
        knowledge_id: Some("search-test".to_string()),
        path_prefix: None,
        exclude_suffixes: vec![],
        exclude_contains: vec![],
        limit: 10,
    };
    
    let session_results = db.search_purpose(&params2).unwrap();
    assert!(session_results.iter().any(|r| r.path.contains("session")));
    
    // Search by path
    let path_results = db.search_paths(&params).unwrap();
    assert!(path_results.iter().any(|r| r.path.contains("auth")));
}

/// Test entity deduplication (same keyword across multiple files)
#[test]
fn test_entity_deduplication() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    let knowledge = Knowledge::new_local("dedup-test", "test/repo", "/path/to/repo");
    db.insert_knowledge(&knowledge).unwrap();
    
    // Create multiple files using same keywords
    for path in &["src/file1.rs", "src/file2.rs", "lib/file3.rs"] {
        let file_input = FileInput {
            knowledge_id: "dedup-test".to_string(),
            relative_path: path.to_string(),
            language: Some("rust".to_string()),
            sha256: format!("sha-{}", path),
            size_bytes: 100,
            purpose: Some("Test file".to_string()),
            summary: None,
            business_context: None,
        };
        let file_id = db.upsert_file(&file_input).unwrap();
        
        // All files use "authentication" keyword
        let kw_id = db.get_or_create_keyword("authentication").unwrap();
        db.link_file_keywords(file_id, &[kw_id]).unwrap();
    }
    
    // Search for authentication - should find all 3 files
    let params = codegraph::db::SearchParams {
        query: "authentication".to_string(),
        knowledge_id: Some("dedup-test".to_string()),
        path_prefix: None,
        exclude_suffixes: vec![],
        exclude_contains: vec![],
        limit: 10,
    };
    
    let results = db.search_keywords(&params).unwrap();
    // Should find files with the keyword
    assert_eq!(results.len(), 3);
}

/// Test file update (diff-aware re-indexing simulation)
#[test]
fn test_file_update_diffs_sha() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    let knowledge = Knowledge::new_local("diff-test", "test/repo", "/path/to/repo");
    db.insert_knowledge(&knowledge).unwrap();
    
    // Initial file
    let file_input1 = FileInput {
        knowledge_id: "diff-test".to_string(),
        relative_path: "src/main.rs".to_string(),
        language: Some("rust".to_string()),
        sha256: "old-sha".to_string(),
        size_bytes: 100,
        purpose: Some("Old purpose".to_string()),
        summary: Some("Old summary".to_string()),
        business_context: None,
    };
    let file_id = db.upsert_file(&file_input1).unwrap();
    
    // Get SHA - should be old
    let file = db.get_file("diff-test", "src/main.rs").unwrap().unwrap();
    assert_eq!(file.sha256, "old-sha");
    assert_eq!(file.purpose, Some("Old purpose".to_string()));
    
    // Simulate update with new SHA (file changed)
    let file_input2 = FileInput {
        knowledge_id: "diff-test".to_string(),
        relative_path: "src/main.rs".to_string(),
        language: Some("rust".to_string()),
        sha256: "new-sha".to_string(),
        size_bytes: 150,
        purpose: Some("New purpose".to_string()),
        summary: Some("New summary".to_string()),
        business_context: Some("Updated business context".to_string()),
    };
    db.upsert_file(&file_input2).unwrap();
    
    // Verify update
    let file = db.get_file("diff-test", "src/main.rs").unwrap().unwrap();
    assert_eq!(file.sha256, "new-sha");
    assert_eq!(file.purpose, Some("New purpose".to_string()));
    assert_eq!(file.business_context, Some("Updated business context".to_string()));
}

/// Test knowledge isolation (different repos don't interfere)
#[test]
fn test_knowledge_isolation() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    // Create two separate repos
    let knowledge1 = Knowledge::new_local("repo1", "owner/repo1", "/path/repo1");
    let knowledge2 = Knowledge::new_local("repo2", "owner/repo2", "/path/repo2");
    
    db.insert_knowledge(&knowledge1).unwrap();
    db.insert_knowledge(&knowledge2).unwrap();
    
    // Add file to repo1
    let file_input1 = FileInput {
        knowledge_id: "repo1".to_string(),
        relative_path: "auth.rs".to_string(),
        language: Some("typescript".to_string()),
        sha256: "sha1".to_string(),
        size_bytes: 100,
        purpose: Some("Auth for repo1".to_string()),
        summary: None,
        business_context: None,
    };
    db.upsert_file(&file_input1).unwrap();
    
    // Add file to repo2 with same name
    let file_input2 = FileInput {
        knowledge_id: "repo2".to_string(),
        relative_path: "auth.rs".to_string(),
        language: Some("typescript".to_string()),
        sha256: "sha2".to_string(),
        size_bytes: 200,
        purpose: Some("Auth for repo2".to_string()),
        summary: None,
        business_context: None,
    };
    db.upsert_file(&file_input2).unwrap();
    
    // Verify they are separate
    let file1 = db.get_file("repo1", "auth.rs").unwrap().unwrap();
    let file2 = db.get_file("repo2", "auth.rs").unwrap().unwrap();
    
    assert_eq!(file1.knowledge_id, "repo1");
    assert_eq!(file2.knowledge_id, "repo2");
    assert_ne!(file1.sha256, file2.sha256);
    
    // List all knowledge
    let repos = db.list_knowledge().unwrap();
    assert_eq!(repos.len(), 2);
    
    // Get files by knowledge
    let repo1_files = db.get_files_by_knowledge("repo1").unwrap();
    let repo2_files = db.get_files_by_knowledge("repo2").unwrap();
    
    assert_eq!(repo1_files.len(), 1);
    assert_eq!(repo2_files.len(), 1);
}

/// Test file deletion cleanup
#[test]
fn test_delete_cascades() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    let knowledge = Knowledge::new_local("delete-test", "test/repo", "/path/to/repo");
    db.insert_knowledge(&knowledge).unwrap();
    
    // Add file with entities
    let file_input = FileInput {
        knowledge_id: "delete-test".to_string(),
        relative_path: "test.rs".to_string(),
        language: Some("rust".to_string()),
        sha256: "test-sha".to_string(),
        size_bytes: 100,
        purpose: Some("Test file".to_string()),
        summary: None,
        business_context: None,
    };
    let file_id = db.upsert_file(&file_input).unwrap();
    
    // Link entities
    let kw_id = db.get_or_create_keyword("test").unwrap();
    db.link_file_keywords(file_id, &[kw_id]).unwrap();
    
    // Verify file exists
    assert!(db.get_file("delete-test", "test.rs").unwrap().is_some());
    
    // Delete knowledge
    db.delete_knowledge("delete-test").unwrap();
    
    // Verify file is gone
    assert!(db.get_file("delete-test", "test.rs").unwrap().is_none());
    
    // Verify knowledge is gone
    assert!(db.get_knowledge("delete-test").unwrap().is_none());
}

/// Test SHAS tracking for diff-aware indexing
#[test]
fn test_sha_tracking() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    let knowledge = Knowledge::new_local("sha-test", "test/repo", "/path/to/repo");
    db.insert_knowledge(&knowledge).unwrap();
    
    // Add multiple files
    let files = vec!["file1.rs", "file2.rs", "file3.rs"];
    for (i, path) in files.iter().enumerate() {
        let file_input = FileInput {
            knowledge_id: "sha-test".to_string(),
            relative_path: path.to_string(),
            language: Some("rust".to_string()),
            sha256: format!("sha-{}", i),
            size_bytes: 100,
            purpose: Some("Test".to_string()),
            summary: None,
            business_context: None,
        };
        db.upsert_file(&file_input).unwrap();
    }
    
    // Get all SHAs
    let shas = db.get_file_shas("sha-test").unwrap();
    assert_eq!(shas.len(), 3);
    assert_eq!(shas.get("file1.rs"), Some(&"sha-0".to_string()));
    assert_eq!(shas.get("file2.rs"), Some(&"sha-1".to_string()));
    
    // Simulate file deletion (cleanup old files)
    db.delete_files_not_in("sha-test", &["file1.rs".to_string()]).unwrap();
    
    let shas = db.get_file_shas("sha-test").unwrap();
    assert_eq!(shas.len(), 1);
    assert!(shas.contains_key("file1.rs"));
}
