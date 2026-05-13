//! Database tests

use codegraph::db::{Database, Knowledge, KnowledgeState, FileInput, KnowledgeSource};
use tempfile::tempdir;

#[test]
fn test_insert_and_get_knowledge() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    let knowledge = Knowledge::new_github(
        "test-id-123",
        "test/repo",
        "https://github.com/test/repo",
        Some("main"),
    );
    
    db.insert_knowledge(&knowledge).unwrap();
    
    let retrieved = db.get_knowledge("test-id-123").unwrap().unwrap();
    assert_eq!(retrieved.id, "test-id-123");
    assert_eq!(retrieved.repo_name, "test/repo");
    assert_eq!(retrieved.state, KnowledgeState::Created);
    
    match retrieved.source {
        KnowledgeSource::Github { url, branch } => {
            assert_eq!(url, "https://github.com/test/repo");
            assert_eq!(branch, Some("main".to_string()));
        }
        _ => panic!("Expected Github source"),
    }
}

#[test]
fn test_update_knowledge_state() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    let knowledge = Knowledge::new_github(
        "test-id-456",
        "test/repo2",
        "https://github.com/test/repo2",
        None,
    );
    
    db.insert_knowledge(&knowledge).unwrap();
    
    db.update_knowledge_state("test-id-456", KnowledgeState::Processing).unwrap();
    
    let retrieved = db.get_knowledge("test-id-456").unwrap().unwrap();
    assert_eq!(retrieved.state, KnowledgeState::Processing);
}

#[test]
fn test_list_knowledge() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    for i in 0..5 {
        let knowledge = Knowledge::new_local(
            &format!("id-{}", i),
            &format!("repo/{}", i),
            &format!("/path/to/repo/{}", i),
        );
        db.insert_knowledge(&knowledge).unwrap();
    }
    
    let repos = db.list_knowledge().unwrap();
    assert_eq!(repos.len(), 5);
}

#[test]
fn test_upsert_file() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    let knowledge = Knowledge::new_local(
        "file-test-id",
        "test/repo",
        "/path/to/repo",
    );
    db.insert_knowledge(&knowledge).unwrap();
    
    let file_input = FileInput {
        knowledge_id: "file-test-id".to_string(),
        relative_path: "src/main.rs".to_string(),
        language: Some("rust".to_string()),
        sha256: "abc123".to_string(),
        size_bytes: 100,
        purpose: Some("Main entry point".to_string()),
        summary: Some("This is the main file".to_string()),
        business_context: Some("Core application logic".to_string()),
    };
    
    let file_id = db.upsert_file(&file_input).unwrap();
    assert!(file_id > 0);
    
    let retrieved = db.get_file("file-test-id", "src/main.rs").unwrap().unwrap();
    assert_eq!(retrieved.language, Some("rust".to_string()));
    assert_eq!(retrieved.purpose, Some("Main entry point".to_string()));
}

#[test]
fn test_file_update() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    let knowledge = Knowledge::new_local(
        "update-test-id",
        "test/repo",
        "/path/to/repo",
    );
    db.insert_knowledge(&knowledge).unwrap();
    
    let file_input = FileInput {
        knowledge_id: "update-test-id".to_string(),
        relative_path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        sha256: "old-sha".to_string(),
        size_bytes: 50,
        purpose: Some("Old purpose".to_string()),
        summary: Some("Old summary".to_string()),
        business_context: None,
    };
    
    db.upsert_file(&file_input).unwrap();
    
    // Update with new SHA
    let file_input2 = FileInput {
        knowledge_id: "update-test-id".to_string(),
        relative_path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        sha256: "new-sha".to_string(),
        size_bytes: 100,
        purpose: Some("New purpose".to_string()),
        summary: Some("New summary".to_string()),
        business_context: Some("New context".to_string()),
    };
    
    db.upsert_file(&file_input2).unwrap();
    
    let retrieved = db.get_file("update-test-id", "src/lib.rs").unwrap().unwrap();
    assert_eq!(retrieved.sha256, "new-sha");
    assert_eq!(retrieved.purpose, Some("New purpose".to_string()));
}

#[test]
fn test_delete_knowledge() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    let knowledge = Knowledge::new_local(
        "delete-test-id",
        "test/repo",
        "/path/to/repo",
    );
    db.insert_knowledge(&knowledge).unwrap();
    
    db.delete_knowledge("delete-test-id").unwrap();
    
    let retrieved = db.get_knowledge("delete-test-id").unwrap();
    assert!(retrieved.is_none());
}

#[test]
fn test_get_files_by_knowledge() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    let knowledge = Knowledge::new_local(
        "files-test-id",
        "test/repo",
        "/path/to/repo",
    );
    db.insert_knowledge(&knowledge).unwrap();
    
    for i in 0..3 {
        let file_input = FileInput {
            knowledge_id: "files-test-id".to_string(),
            relative_path: format!("src/file{}.rs", i),
            language: Some("rust".to_string()),
            sha256: format!("sha{}", i),
            size_bytes: 100,
            purpose: Some(format!("File {}", i)),
            summary: None,
            business_context: None,
        };
        db.upsert_file(&file_input).unwrap();
    }
    
    let files = db.get_files_by_knowledge("files-test-id").unwrap();
    assert_eq!(files.len(), 3);
}

#[test]
fn test_file_shas() {
    let dir = tempdir().unwrap();
    let db = Database::open_path(dir.path().join("test.db").as_path()).unwrap();
    
    let knowledge = Knowledge::new_local(
        "sha-test-id",
        "test/repo",
        "/path/to/repo",
    );
    db.insert_knowledge(&knowledge).unwrap();
    
    let file_input = FileInput {
        knowledge_id: "sha-test-id".to_string(),
        relative_path: "test.rs".to_string(),
        language: Some("rust".to_string()),
        sha256: "test-sha-456".to_string(),
        size_bytes: 100,
        purpose: Some("Test file".to_string()),
        summary: None,
        business_context: None,
    };
    db.upsert_file(&file_input).unwrap();
    
    let shas = db.get_file_shas("sha-test-id").unwrap();
    assert_eq!(shas.get("test.rs"), Some(&"test-sha-456".to_string()));
}
