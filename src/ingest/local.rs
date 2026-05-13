//! Local directory ingestion

use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tracing::{info, warn};

use crate::config::load_config;
use crate::db::{Database, KnowledgeState, LinkType};
use crate::queue::{QueueManager, new_local_ingest_job, JobPriority};
use crate::ingest::{scan_directory, analyze_file, compute_sha256, detect_language};
use crate::llm::LlmClient;
use uuid::Uuid;

/// Result of processing a single file
type FileProcessResult = Result<(String, String), (String, String)>;

/// Ingest a local directory
/// 
/// If `force` is true, re-analyzes all files regardless of SHA256 (ignores unchanged check)
pub async fn ingest_path(path: &Path, force: bool) -> Result<()> {
    let config = load_config()?;
    config.validate().map_err(|e| anyhow::anyhow!(e))?;

    let path_str = path.to_string_lossy().to_string();
    // Use the original path string to match stored repos (don't canonicalize)
    let lookup_path = path_str.clone();
    
    let repo_name = path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "local".to_string());

    let db = Database::open()?;
    
    // Check if this path is already indexed
    let existing_knowledge = db.get_knowledge_by_path(&lookup_path)?;
    
    let knowledge_id = if let Some(existing) = existing_knowledge {
        if force {
            info!("Force re-indexing repo {} at {}", existing.id, path_str);
        } else {
            info!("Found existing repo {} for path {}", existing.id, path_str);
        }
        existing.id
    } else {
        let new_id = Uuid::new_v4().to_string();
        info!("Creating new repo {} for path {}", new_id, path_str);
        
        // Create knowledge record
        db.insert_knowledge(&crate::db::Knowledge::new_local(
            &new_id,
            &repo_name,
            &path_str,
        ))?;
        new_id
    };

    db.update_knowledge_state(&knowledge_id, KnowledgeState::Queued)?;

    // Enqueue job
    let manager = QueueManager::open()?;
    let job = new_local_ingest_job(&knowledge_id, &path_str, JobPriority::Normal);
    manager.enqueue(&job)?;

    // Update state
    db.update_knowledge_state(&knowledge_id, KnowledgeState::Ingested)?;

    // Scan files
    info!("Scanning files...");
    let files = scan_directory(path).await?;
    
    // Get existing SHA256 map for diff-aware re-indexing
    let existing_shas = if force {
        info!("Force mode: ignoring existing file hashes, will re-analyze all files");
        std::collections::HashMap::new()
    } else {
        let shas = db.get_file_shas(&knowledge_id)?;
        info!("Found {} existing file hashes in database", shas.len());
        shas
    };
    
    db.update_knowledge_progress(&knowledge_id, files.len() as i32, 0)?;

    // Share database across concurrent tasks (internal mutex serializes writes)
    let db = Arc::new(db);

    // Create LLM client
    let _llm = LlmClient::new(&config)?;

    // Update state
    db.update_knowledge_state(&knowledge_id, KnowledgeState::Processing)?;

    // Analyze files concurrently with semaphore-gated concurrency
    let semaphore = Arc::new(Semaphore::new(config.concurrency));
    info!("Starting ingestion with concurrency limit: {} (DB writes serialized){}", 
          config.concurrency, if force { " [FORCE MODE]" } else { "" });

    // Separate files into unchanged vs needs processing
    let mut unchanged_count = 0;
    let to_process: Vec<_> = files.iter().filter(|file| {
        if force {
            // Force mode: process all files
            true
        } else {
            let file_sha = compute_sha256(&file.content);
            match existing_shas.get(&file.relative_path) {
                Some(existing) if existing == &file_sha => {
                    unchanged_count += 1;
                    false
                }
                _ => true
            }
        }
    }).cloned().collect();
    let skipped = unchanged_count;

    let processed_count = to_process.len();
    info!("Processing {} files ({} unchanged, {} to analyze)", 
          files.len(), skipped, processed_count);

    // Spawn tasks for each file with semaphore gating
    let mut handles = Vec::new();
    for file in to_process {
        let sem = semaphore.clone();
        let llm = LlmClient::new(&config)?;
        let knowledge_id = knowledge_id.clone();
        let db = db.clone();

        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.expect("Semaphore closed");
            
            let file_sha = compute_sha256(&file.content);
            
            let analysis = match analyze_file(&llm, &file.relative_path, &file.content).await {
                Ok(a) => a,
                Err(e) => {
                    warn!("Failed to analyze {}: {}", file.relative_path, e);
                    return Err((file.relative_path.clone(), e.to_string()));
                }
            };

            let language = if !analysis.language.is_empty() && analysis.language != "unknown" {
                Some(analysis.language.clone())
            } else {
                detect_language(&file.relative_path)
            };

            // Save to database (DB mutex serializes writes)
            let input = crate::db::FileInput {
                knowledge_id: knowledge_id.clone(),
                relative_path: file.relative_path.clone(),
                language,
                sha256: file_sha.clone(),
                size_bytes: file.size_bytes as i64,
                purpose: Some(analysis.analysis.purpose),
                summary: Some(analysis.analysis.summary),
                business_context: Some(analysis.analysis.business_context),
            };

            let file_id = db.upsert_file(&input).map_err(|e| (file.relative_path.clone(), e.to_string()))?;
            db.clear_file_links(file_id).map_err(|e| (file.relative_path.clone(), e.to_string()))?;

            // Link entities
            for keyword in &analysis.analysis.keywords {
                if let Ok(kid) = db.get_or_create_keyword(keyword) {
                    let _ = db.link_file_keywords(file_id, &[kid], LinkType::References);
                }
            }

            for class in &analysis.analysis.classes_defined {
                if let Ok(cid) = db.get_or_create_class(class) {
                    let _ = db.link_file_classes(file_id, &[cid], LinkType::Defines);
                }
            }

            for class in &analysis.analysis.classes_used {
                if let Ok(cid) = db.get_or_create_class(&extract_entity_name(class)) {
                    let _ = db.link_file_classes(file_id, &[cid], LinkType::References);
                }
            }

            for func in &analysis.analysis.functions_defined {
                if let Ok(fid) = db.get_or_create_function(func) {
                    let _ = db.link_file_functions(file_id, &[fid], LinkType::Defines);
                }
            }

            for func in &analysis.analysis.functions_used {
                if let Ok(fid) = db.get_or_create_function(&extract_entity_name(func)) {
                    let _ = db.link_file_functions(file_id, &[fid], LinkType::References);
                }
            }

            for module in &analysis.analysis.modules_defined {
                if let Ok(mid) = db.get_or_create_module(module, false) {
                    let _ = db.link_file_imports_internal(file_id, &[mid], LinkType::Defines);
                }
            }

            for imp in &analysis.analysis.modules_imported {
                if let Ok(mid) = db.get_or_create_module(imp, false) {
                    let _ = db.link_file_imports_internal(file_id, &[mid], LinkType::References);
                }
            }

            for imp in &analysis.analysis.modules_external {
                if let Ok(mid) = db.get_or_create_module(imp, true) {
                    let _ = db.link_file_imports_external(file_id, &[mid], LinkType::References);
                }
            }

            Ok((file.relative_path.clone(), file_sha.clone()))
        });

        handles.push(handle);
    }

    // Wait for all tasks and collect results
    let mut processed = 0;
    let mut errors = 0;
    for handle in handles {
        match handle.await {
            Ok(Ok((_path, _))) => {
                processed += 1;
                if processed % 10 == 0 {
                    db.update_knowledge_progress(&knowledge_id, processed_count as i32, processed as i32)?;
                }
            }
            Ok(Err((path, err))) => {
                errors += 1;
                warn!("Failed to process {}: {}", path, err);
            }
            Err(e) => {
                errors += 1;
                warn!("Task failed: {}", e);
            }
        }
    }

    // Clean up deleted files
    let current_paths: Vec<String> = files.iter().map(|f| f.relative_path.clone()).collect();
    let deleted = db.delete_files_not_in(&knowledge_id, &current_paths)?;
    if deleted > 0 {
        info!("Removed {} deleted files from index", deleted);
    }

    // Final state
    db.update_knowledge_progress(&knowledge_id, processed_count as i32, processed as i32)?;
    db.update_knowledge_state(&knowledge_id, KnowledgeState::Processed)?;
    db.update_knowledge_file_count(&knowledge_id, processed as i32)?;

    info!("Ingested {} files, skipped {} unchanged, {} errors from {}", 
          processed, skipped, errors, path_str);

    Ok(())
}

/// Extract just the entity name from entries like "Name: description"
/// Language-agnostic: works with any format that uses ":" as separator
fn extract_entity_name(entry: &str) -> String {
    entry.split(':').next().unwrap_or(entry).trim().to_string()
}
