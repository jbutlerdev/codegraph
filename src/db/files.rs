//! File CRUD operations

use crate::db::Database;
use crate::error::{Error, Result};
use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};

/// File record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub id: i64,
    pub knowledge_id: String,
    pub relative_path: String,
    pub language: Option<String>,
    pub sha256: String,
    pub size_bytes: i64,
    pub purpose: Option<String>,
    pub summary: Option<String>,
    pub business_context: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Input for file creation/update
#[derive(Debug, Clone)]
pub struct FileInput {
    pub knowledge_id: String,
    pub relative_path: String,
    pub language: Option<String>,
    pub sha256: String,
    pub size_bytes: i64,
    pub purpose: Option<String>,
    pub summary: Option<String>,
    pub business_context: Option<String>,
}

/// File analysis from LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAnalysis {
    pub purpose: String,
    pub summary: String,
    pub business_context: String,
    // Entities defined in this file
    pub classes_defined: Vec<String>,
    pub functions_defined: Vec<String>,
    pub keywords: Vec<String>,
    // Entities referenced/imported from this file
    pub classes_used: Vec<String>,
    pub functions_used: Vec<String>,
    // Modules: defines = this IS the module, references = imports this module
    pub modules_defined: Vec<String>,
    pub modules_imported: Vec<String>,
    pub modules_external: Vec<String>,
}

impl Default for FileAnalysis {
    fn default() -> Self {
        Self {
            purpose: String::new(),
            summary: String::new(),
            business_context: String::new(),
            classes_defined: Vec::new(),
            functions_defined: Vec::new(),
            keywords: Vec::new(),
            classes_used: Vec::new(),
            functions_used: Vec::new(),
            modules_defined: Vec::new(),
            modules_imported: Vec::new(),
            modules_external: Vec::new(),
        }
    }
}

/// Entity relationship types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LinkType {
    Defines,
    References,
}

impl std::fmt::Display for LinkType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkType::Defines => write!(f, "defines"),
            LinkType::References => write!(f, "references"),
        }
    }
}

impl Database {
    /// Insert or update a file
    pub fn upsert_file(&self, input: &FileInput) -> Result<i64> {
        let conn = self.conn.lock();
        let now = Utc::now().to_rfc3339();

        conn.execute(
            r#"INSERT INTO files (knowledge_id, relative_path, language, sha256, size_bytes, purpose, summary, business_context, created_at, updated_at)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
               ON CONFLICT(knowledge_id, relative_path) DO UPDATE SET
                   language = excluded.language,
                   sha256 = excluded.sha256,
                   size_bytes = excluded.size_bytes,
                   purpose = excluded.purpose,
                   summary = excluded.summary,
                   business_context = excluded.business_context,
                   updated_at = excluded.updated_at"#,
            params![
                input.knowledge_id,
                input.relative_path,
                input.language,
                input.sha256,
                input.size_bytes,
                input.purpose,
                input.summary,
                input.business_context,
                now,
                now,
            ],
        ).map_err(Error::Database)?;

        let id = conn.last_insert_rowid();
        Ok(id)
    }

    /// Get a file by knowledge ID and path
    pub fn get_file(&self, knowledge_id: &str, relative_path: &str) -> Result<Option<File>> {
        let conn = self.conn.lock();

        let result = conn.query_row(
            "SELECT id, knowledge_id, relative_path, language, sha256, size_bytes, purpose, summary, business_context, created_at, updated_at FROM files WHERE knowledge_id = ?1 AND relative_path = ?2",
            params![knowledge_id, relative_path],
            |row| Ok(File {
                id: row.get(0)?,
                knowledge_id: row.get(1)?,
                relative_path: row.get(2)?,
                language: row.get(3)?,
                sha256: row.get(4)?,
                size_bytes: row.get(5)?,
                purpose: row.get(6)?,
                summary: row.get(7)?,
                business_context: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            }),
        );

        match result {
            Ok(f) => Ok(Some(f)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(Error::Database(e)),
        }
    }

    /// Get files by knowledge ID
    pub fn get_files_by_knowledge(&self, knowledge_id: &str) -> Result<Vec<File>> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            "SELECT id, knowledge_id, relative_path, language, sha256, size_bytes, purpose, summary, business_context, created_at, updated_at FROM files WHERE knowledge_id = ?1 ORDER BY relative_path"
        ).map_err(Error::Database)?;

        let rows = stmt.query_map(params![knowledge_id], |row| {
            Ok(File {
                id: row.get(0)?,
                knowledge_id: row.get(1)?,
                relative_path: row.get(2)?,
                language: row.get(3)?,
                sha256: row.get(4)?,
                size_bytes: row.get(5)?,
                purpose: row.get(6)?,
                summary: row.get(7)?,
                business_context: row.get(8)?,
                created_at: row.get(9)?,
                updated_at: row.get(10)?,
            })
        }).map_err(Error::Database)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(Error::Database)?);
        }
        Ok(results)
    }

    /// Get file SHA256 map for a knowledge (for diff-aware indexing)
    pub fn get_file_shas(&self, knowledge_id: &str) -> Result<std::collections::HashMap<String, String>> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            "SELECT relative_path, sha256 FROM files WHERE knowledge_id = ?1"
        ).map_err(Error::Database)?;

        let rows = stmt.query_map(params![knowledge_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).map_err(Error::Database)?;

        let mut map = std::collections::HashMap::new();
        for row in rows {
            let (path, sha) = row.map_err(Error::Database)?;
            map.insert(path, sha);
        }
        Ok(map)
    }

    /// Delete files not in the given paths (for cleanup after re-index)
    pub fn delete_files_not_in(&self, knowledge_id: &str, paths: &[String]) -> Result<usize> {
        let conn = self.conn.lock();

        if paths.is_empty() {
            let count = conn.execute(
                "DELETE FROM files WHERE knowledge_id = ?1",
                params![knowledge_id],
            ).map_err(Error::Database)?;
            return Ok(count);
        }

        // Build placeholders for IN clause
        let placeholders: Vec<&str> = paths.iter().map(|_| "?").collect();
        let query = format!(
            "DELETE FROM files WHERE knowledge_id = ?1 AND relative_path NOT IN ({})",
            placeholders.join(", ")
        );

        let mut params_vec: Vec<&dyn rusqlite::ToSql> = vec![&knowledge_id];
        for path in paths {
            params_vec.push(path);
        }

        let count = conn.execute(&query, params_vec.as_slice()).map_err(Error::Database)?;
        Ok(count)
    }

    /// Count files for a knowledge
    pub fn count_files(&self, knowledge_id: &str) -> Result<i32> {
        let conn = self.conn.lock();

        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM files WHERE knowledge_id = ?1",
            params![knowledge_id],
            |row| row.get(0),
        ).map_err(Error::Database)?;

        Ok(count)
    }
}
