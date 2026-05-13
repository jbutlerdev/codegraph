//! GitHub repository ingestion

use anyhow::{Context, Result};
use std::path::Path;
use tracing::{info, error};

use crate::config::load_config;
use crate::config::repos_dir;
use crate::db::{Database, KnowledgeState, LinkType};
use crate::queue::{QueueManager, new_github_index_job, new_github_pull_job, JobPriority};
use crate::ingest::{scan_directory, analyze_file, compute_sha256, detect_language};
use crate::llm::LlmClient;
use uuid::Uuid;

/// Clone URL to repo name
fn extract_repo_name(url: &str) -> String {
    url.trim_end_matches(".git")
        .rsplit('/')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

/// Clone a GitHub repository
pub fn clone_repo(url: &str, branch: Option<&str>, token: Option<&str>, target_dir: &Path) -> Result<String> {
    info!("Cloning {} to {:?}", url, target_dir);

    // Build fetch URL with token if provided
    let fetch_url = if let Some(t) = token {
        if url.starts_with("https://") {
            url.replace("https://", &format!("https://{}", t))
        } else {
            url.to_string()
        }
    } else {
        url.to_string()
    };

    let opts = git2::FetchOptions::new();

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(opts);

    if let Some(b) = branch {
        builder.branch(b);
    }

    let repo = builder.clone(&fetch_url, target_dir)
        .context("Failed to clone repository")?;

    // Get the commit hash
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    let commit_hash = commit.id().to_string();

    info!("Cloned {} at commit {}", url, commit_hash);

    Ok(commit_hash)
}

/// Index a GitHub repository
pub async fn index_repo(url: &str, branch: Option<&str>, token: Option<&str>) -> Result<()> {
    let config = load_config()?;
    config.validate().map_err(|e| anyhow::anyhow!(e))?;

    // Generate knowledge ID
    let knowledge_id = Uuid::new_v4().to_string();
    let repo_name = extract_repo_name(url);

    info!("Indexing repository: {} ({})", repo_name, knowledge_id);

    // Create knowledge record
    let db = Database::open()?;
    db.insert_knowledge(&crate::db::Knowledge::new_github(
        &knowledge_id,
        &repo_name,
        url,
        branch,
    ))?;
    db.update_knowledge_state(&knowledge_id, KnowledgeState::Queued)?;

    // Enqueue job
    let manager = QueueManager::open()?;
    let job = new_github_index_job(&knowledge_id, url, branch, token, JobPriority::Normal);
    manager.enqueue(&job)?;

    // Clone repository
    let repos = repos_dir()?;
    let clone_dir = repos.join(&knowledge_id);
    let _commit_hash = clone_repo(url, branch, token, &clone_dir)?;

    // Update with commit hash
    db.update_knowledge_state(&knowledge_id, KnowledgeState::Ingested)?;

    // Scan files
    info!("Scanning files...");
    let files = scan_directory(&clone_dir).await?;
    db.update_knowledge_progress(&knowledge_id, files.len() as i32, 0)?;

    // Create LLM client
    let llm = LlmClient::new(&config)?;

    // Update state
    db.update_knowledge_state(&knowledge_id, KnowledgeState::Processing)?;

    // Analyze files
    let mut processed = 0;
    for file in &files {
        let analysis = match analyze_file(&llm, &file.relative_path, &file.content).await {
            Ok(a) => a,
            Err(e) => {
                error!("Failed to analyze {}: {}", file.relative_path, e);
                continue;
            }
        };

        let language = if !analysis.language.is_empty() && analysis.language != "unknown" {
            Some(analysis.language)
        } else {
            detect_language(&file.relative_path)
        };

        // Save to database
        let input = crate::db::FileInput {
            knowledge_id: knowledge_id.clone(),
            relative_path: file.relative_path.clone(),
            language,
            sha256: compute_sha256(&file.content),
            size_bytes: file.size_bytes as i64,
            purpose: Some(analysis.analysis.purpose),
            summary: Some(analysis.analysis.summary),
            business_context: Some(analysis.analysis.business_context),
        };

        let file_id = db.upsert_file(&input)?;

        // Link entities with proper relationship types
        for keyword in &analysis.analysis.keywords {
            let kid = db.get_or_create_keyword(keyword)?;
            db.link_file_keywords(file_id, &[kid], LinkType::References)?;
        }

        // Classes DEFINED in this file
        for class in &analysis.analysis.classes_defined {
            let cid = db.get_or_create_class(class)?;
            db.link_file_classes(file_id, &[cid], LinkType::Defines)?;
        }

        // Classes USED/IMPORTED by this file
        for class in &analysis.analysis.classes_used {
            let cid = db.get_or_create_class(&extract_entity_name(class))?;
            db.link_file_classes(file_id, &[cid], LinkType::References)?;
        }

        // Functions DEFINED in this file
        for func in &analysis.analysis.functions_defined {
            let fid = db.get_or_create_function(func)?;
            db.link_file_functions(file_id, &[fid], LinkType::Defines)?;
        }

        // Functions USED/CALLED by this file
        for func in &analysis.analysis.functions_used {
            let fid = db.get_or_create_function(&extract_entity_name(func))?;
            db.link_file_functions(file_id, &[fid], LinkType::References)?;
        }

        // Modules DEFINED by this file
        for module in &analysis.analysis.modules_defined {
            let mid = db.get_or_create_module(module, false)?;
            db.link_file_imports_internal(file_id, &[mid], LinkType::Defines)?;
        }

        // Internal modules IMPORTED by this file
        for imp in &analysis.analysis.modules_imported {
            let mid = db.get_or_create_module(imp, false)?;
            db.link_file_imports_internal(file_id, &[mid], LinkType::References)?;
        }

        // External modules IMPORTED by this file
        for imp in &analysis.analysis.modules_external {
            let mid = db.get_or_create_module(imp, true)?;
            db.link_file_imports_external(file_id, &[mid], LinkType::References)?;
        }

        processed += 1;
        if processed % 10 == 0 {
            db.update_knowledge_progress(&knowledge_id, files.len() as i32, processed as i32)?;
        }
    }

    // Final state
    db.update_knowledge_progress(&knowledge_id, files.len() as i32, processed as i32)?;
    db.update_knowledge_state(&knowledge_id, KnowledgeState::Processed)?;
    db.update_knowledge_file_count(&knowledge_id, processed as i32)?;

    info!("Indexed {} files from {}", processed, repo_name);

    Ok(())
}

/// Pull (re-index) an existing repository
pub async fn pull_repo(knowledge_id: &str) -> Result<()> {
    let db = Database::open()?;

    // Get knowledge record
    let _knowledge = db.get_knowledge(knowledge_id)?
        .ok_or_else(|| anyhow::anyhow!("Knowledge not found: {}", knowledge_id))?;

    // Enqueue job
    let manager = QueueManager::open()?;
    let job = new_github_pull_job(knowledge_id, JobPriority::Normal);
    manager.enqueue(&job)?;

    // Update state
    db.update_knowledge_state(knowledge_id, KnowledgeState::Processing)?;

    info!("Pulling repository: {}", knowledge_id);

    Ok(())
}

/// Extract just the entity name from entries like "Name: description"
fn extract_entity_name(entry: &str) -> String {
    entry.split(':').next().unwrap_or(entry).trim().to_string()
}
