//! Entity tests

use codegraph::db::{Database, Knowledge, FileInput, KnowledgeState, LinkType};
use tempfile::tempdir;

#[test]
fn test_keyword_crud() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    let knowledge = Knowledge::new_local(
        "keyword-test-id",
        "test/repo",
        "/path/to/repo",
    );
    db.insert_knowledge(&knowledge).unwrap();
    
    let file_input = FileInput {
        knowledge_id: "keyword-test-id".to_string(),
        relative_path: "src/main.rs".to_string(),
        language: Some("rust".to_string()),
        sha256: "test-sha".to_string(),
        size_bytes: 100,
        purpose: Some("Test".to_string()),
        summary: None,
        business_context: None,
    };
    
    let file_id = db.upsert_file(&file_input).unwrap();
    
    // Create keywords
    let kid1 = db.get_or_create_keyword("authentication").unwrap();
    let kid2 = db.get_or_create_keyword("Auth").unwrap(); // Should match case-insensitive
    
    // Link to file
    db.link_file_keywords(file_id, &[kid1], LinkType::Defines).unwrap();
    
    // Get file keywords
    let keywords = db.get_file_keywords(file_id).unwrap();
    assert_eq!(keywords.len(), 1);
    assert_eq!(keywords[0], "authentication");
    
    // Test dedup - same keyword created again
    let kid1_again = db.get_or_create_keyword("AUTHENTICATION").unwrap();
    assert_eq!(kid1, kid1_again);
}

#[test]
fn test_class_crud() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    let knowledge = Knowledge::new_local(
        "class-test-id",
        "test/repo",
        "/path/to/repo",
    );
    db.insert_knowledge(&knowledge).unwrap();
    
    let file_input = FileInput {
        knowledge_id: "class-test-id".to_string(),
        relative_path: "src/main.rs".to_string(),
        language: Some("rust".to_string()),
        sha256: "test-sha".to_string(),
        size_bytes: 100,
        purpose: Some("Test".to_string()),
        summary: None,
        business_context: None,
    };
    
    let file_id = db.upsert_file(&file_input).unwrap();
    
    // Create class
    let cid = db.get_or_create_class("AuthService (~L12-58): handles login").unwrap();
    db.link_file_classes(file_id, &[cid], LinkType::Defines).unwrap();
    
    // Get file classes
    let classes = db.get_file_classes(file_id).unwrap();
    assert_eq!(classes.len(), 1);
    assert!(classes[0].contains("AuthService"));
}

#[test]
fn test_function_crud() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    let knowledge = Knowledge::new_local(
        "func-test-id",
        "test/repo",
        "/path/to/repo",
    );
    db.insert_knowledge(&knowledge).unwrap();
    
    let file_input = FileInput {
        knowledge_id: "func-test-id".to_string(),
        relative_path: "src/main.rs".to_string(),
        language: Some("rust".to_string()),
        sha256: "test-sha".to_string(),
        size_bytes: 100,
        purpose: Some("Test".to_string()),
        summary: None,
        business_context: None,
    };
    
    let file_id = db.upsert_file(&file_input).unwrap();
    
    // Create function
    let fid = db.get_or_create_function("handle_login (~L60-100): authenticates user").unwrap();
    db.link_file_functions(file_id, &[fid], LinkType::Defines).unwrap();
    
    // Get file functions
    let functions = db.get_file_functions(file_id).unwrap();
    assert_eq!(functions.len(), 1);
    assert!(functions[0].contains("handle_login"));
}

#[test]
fn test_module_crud() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    let knowledge = Knowledge::new_local(
        "module-test-id",
        "test/repo",
        "/path/to/repo",
    );
    db.insert_knowledge(&knowledge).unwrap();
    
    let file_input = FileInput {
        knowledge_id: "module-test-id".to_string(),
        relative_path: "src/main.rs".to_string(),
        language: Some("rust".to_string()),
        sha256: "test-sha".to_string(),
        size_bytes: 100,
        purpose: Some("Test".to_string()),
        summary: None,
        business_context: None,
    };
    
    let file_id = db.upsert_file(&file_input).unwrap();
    
    // Create internal module
    let mid1 = db.get_or_create_module("./auth", false).unwrap();
    db.link_file_imports_internal(file_id, &[mid1], LinkType::Defines).unwrap();
    
    // Create external module
    let mid2 = db.get_or_create_module("bcrypt", true).unwrap();
    db.link_file_imports_external(file_id, &[mid2], LinkType::References).unwrap();
    
    // Get file imports
    let (internal, external) = db.get_file_imports(file_id).unwrap();
    assert_eq!(internal.len(), 1);
    assert_eq!(internal[0], "./auth");
    assert_eq!(external.len(), 1);
    assert_eq!(external[0], "bcrypt");
}

#[test]
fn test_clear_file_links() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    let knowledge = Knowledge::new_local(
        "clear-test-id",
        "test/repo",
        "/path/to/repo",
    );
    db.insert_knowledge(&knowledge).unwrap();
    
    let file_input = FileInput {
        knowledge_id: "clear-test-id".to_string(),
        relative_path: "src/main.rs".to_string(),
        language: Some("rust".to_string()),
        sha256: "test-sha".to_string(),
        size_bytes: 100,
        purpose: Some("Test".to_string()),
        summary: None,
        business_context: None,
    };
    
    let file_id = db.upsert_file(&file_input).unwrap();
    
    // Add links
    let kid = db.get_or_create_keyword("test").unwrap();
    db.link_file_keywords(file_id, &[kid], LinkType::Defines).unwrap();
    
    // Verify link exists
    let keywords = db.get_file_keywords(file_id).unwrap();
    assert_eq!(keywords.len(), 1);
    
    // Clear links
    db.clear_file_links(file_id).unwrap();
    
    // Verify links are gone
    let keywords = db.get_file_keywords(file_id).unwrap();
    assert_eq!(keywords.len(), 0);
}
