//! Knowledge (repository) CRUD operations

use crate::db::Database;
use crate::error::{Error, Result};
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};

/// Knowledge state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KnowledgeState {
    Created,
    Queued,
    Ingested,
    Processing,
    Processed,
    Failed,
}

impl std::fmt::Display for KnowledgeState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KnowledgeState::Created => write!(f, "created"),
            KnowledgeState::Queued => write!(f, "queued"),
            KnowledgeState::Ingested => write!(f, "ingested"),
            KnowledgeState::Processing => write!(f, "processing"),
            KnowledgeState::Processed => write!(f, "processed"),
            KnowledgeState::Failed => write!(f, "failed"),
        }
    }
}

/// Source of knowledge
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum KnowledgeSource {
    Github { url: String, branch: Option<String> },
    Local { path: String },
}

/// Knowledge record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Knowledge {
    pub id: String,
    pub repo_name: String,
    pub source: KnowledgeSource,
    pub state: KnowledgeState,
    pub file_count: i32,
    pub total_files: i32,
    pub processed_files: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl Knowledge {
    /// Create a new knowledge record for GitHub
    pub fn new_github(id: &str, repo_name: &str, url: &str, branch: Option<&str>) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id: id.to_string(),
            repo_name: repo_name.to_string(),
            source: KnowledgeSource::Github {
                url: url.to_string(),
                branch: branch.map(|s| s.to_string()),
            },
            state: KnowledgeState::Created,
            file_count: 0,
            total_files: 0,
            processed_files: 0,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Create a new knowledge record for local path
    pub fn new_local(id: &str, repo_name: &str, path: &str) -> Self {
        let now = Utc::now().to_rfc3339();
        Self {
            id: id.to_string(),
            repo_name: repo_name.to_string(),
            source: KnowledgeSource::Local {
                path: path.to_string(),
            },
            state: KnowledgeState::Created,
            file_count: 0,
            total_files: 0,
            processed_files: 0,
            created_at: now.clone(),
            updated_at: now,
        }
    }
}

impl Database {
    /// Insert a new knowledge record
    pub fn insert_knowledge(&self, knowledge: &Knowledge) -> Result<()> {
        let conn = self.conn.lock();

        let (source_kind, source_url, source_path, branch) = match &knowledge.source {
            KnowledgeSource::Github { url, branch: b } => ("github", Some(url.as_str()), None::<&str>, b.as_deref()),
            KnowledgeSource::Local { path } => ("local", None::<&str>, Some(path.as_str()), None),
        };

        conn.execute(
            r#"INSERT INTO knowledge (id, repo_name, source_kind, source_url, source_path, branch, state, file_count, total_files, processed_files, created_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)"#,
            params![
                knowledge.id,
                knowledge.repo_name,
                source_kind,
                source_url,
                source_path,
                branch,
                knowledge.state.to_string(),
                knowledge.file_count,
                knowledge.total_files,
                knowledge.processed_files,
                knowledge.created_at,
                knowledge.updated_at,
            ],
        ).map_err(Error::Database)?;

        Ok(())
    }

    /// Get a knowledge record by ID
    pub fn get_knowledge(&self, id: &str) -> Result<Option<Knowledge>> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            "SELECT id, repo_name, source_kind, source_url, source_path, branch, state, file_count, total_files, processed_files, created_at, updated_at FROM knowledge WHERE id = ?1"
        ).map_err(Error::Database)?;

        let result = stmt.query_row(params![id], |row| {
            Self::row_to_knowledge(row)
        });

        match result {
            Ok(k) => Ok(Some(k)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::Database(e)),
        }
    }

    /// Get a local knowledge record by path
    pub fn get_knowledge_by_path(&self, path: &str) -> Result<Option<Knowledge>> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            "SELECT id, repo_name, source_kind, source_url, source_path, branch, state, file_count, total_files, processed_files, created_at, updated_at FROM knowledge WHERE source_kind = 'local' AND source_path = ?1"
        ).map_err(Error::Database)?;

        let result = stmt.query_row(params![path], |row| {
            Self::row_to_knowledge(row)
        });

        match result {
            Ok(k) => Ok(Some(k)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::Database(e)),
        }
    }

    /// Helper to convert a row to Knowledge
    fn row_to_knowledge(row: &rusqlite::Row) -> rusqlite::Result<Knowledge> {
        let source_kind: String = row.get(2)?;
        let source_url: Option<String> = row.get(3)?;
        let source_path: Option<String> = row.get(4)?;
        let branch: Option<String> = row.get(5)?;
        let state_str: String = row.get(6)?;

        let source = if source_kind == "github" {
            KnowledgeSource::Github {
                url: source_url.unwrap_or_default(),
                branch,
            }
        } else {
            KnowledgeSource::Local {
                path: source_path.unwrap_or_default(),
            }
        };

        let state = match state_str.as_str() {
            "created" => KnowledgeState::Created,
            "queued" => KnowledgeState::Queued,
            "ingested" => KnowledgeState::Ingested,
            "processing" => KnowledgeState::Processing,
            "processed" => KnowledgeState::Processed,
            "failed" => KnowledgeState::Failed,
            _ => KnowledgeState::Created,
        };

        Ok(Knowledge {
            id: row.get(0)?,
            repo_name: row.get(1)?,
            source,
            state,
            file_count: row.get(7)?,
            total_files: row.get(8)?,
            processed_files: row.get(9)?,
            created_at: row.get(10)?,
            updated_at: row.get(11)?,
        })
    }

    /// List all knowledge records
    pub fn list_knowledge(&self) -> Result<Vec<Knowledge>> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            "SELECT id, repo_name, source_kind, source_url, source_path, branch, state, file_count, total_files, processed_files, created_at, updated_at FROM knowledge ORDER BY updated_at DESC"
        ).map_err(Error::Database)?;

        let rows = stmt.query_map([], Self::row_to_knowledge).map_err(Error::Database)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(Error::Database)?);
        }
        Ok(results)
    }

    /// Update knowledge state
    pub fn update_knowledge_state(&self, id: &str, state: KnowledgeState) -> Result<()> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();

        let rows_affected = conn.execute(
            "UPDATE knowledge SET state = ?1, updated_at = ?2 WHERE id = ?3",
            params![state.to_string(), now, id],
        ).map_err(Error::Database)?;

        tracing::debug!("update_knowledge_state: id={}, state={}, rows_affected={}", id, state, rows_affected);
        Ok(())
    }

    /// Update knowledge file count
    pub fn update_knowledge_file_count(&self, id: &str, count: i32) -> Result<()> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();
        let rows_affected = conn.execute(
            "UPDATE knowledge SET file_count = ?1, updated_at = ?2 WHERE id = ?3",
            params![count, now, id],
        ).map_err(Error::Database)?;
        tracing::debug!("update_knowledge_file_count: id={}, count={}, rows_affected={}", id, count, rows_affected);
        Ok(())
    }

    /// Update knowledge progress
    pub fn update_knowledge_progress(&self, id: &str, total: i32, processed: i32) -> Result<()> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();

        let rows_affected = conn.execute(
            "UPDATE knowledge SET total_files = ?1, processed_files = ?2, updated_at = ?3 WHERE id = ?4",
            params![total, processed, now, id],
        ).map_err(Error::Database)?;

        tracing::debug!("update_knowledge_progress: id={}, total={}, processed={}, rows_affected={}", id, total, processed, rows_affected);
        Ok(())
    }

    /// Delete a knowledge record
    pub fn delete_knowledge(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM knowledge WHERE id = ?1", params![id]).map_err(Error::Database)?;
        Ok(())
    }
}
