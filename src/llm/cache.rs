//! LLM response caching

use anyhow::Result;
use rusqlite::{params};
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::db::Database;

/// Cache entry
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub content: String,
    pub model: String,
}

/// Cache key computation
pub fn compute_cache_key(prompt: &str, model: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prompt.as_bytes());
    hasher.update(model.as_bytes());
    hex::encode(hasher.finalize())
}

/// LLM cache
pub struct LlmCache {
    db: Arc<Database>,
    enabled: bool,
}

impl LlmCache {
    /// Create a new cache
    pub fn new(db: Arc<Database>, enabled: bool) -> Self {
        Self { db, enabled }
    }

    /// Get cached response
    pub fn get(&self, key: &str) -> Result<Option<CacheEntry>> {
        if !self.enabled {
            return Ok(None);
        }

        let conn = self.db.conn.lock();

        let result = conn.query_row(
            "SELECT content, model FROM llm_cache WHERE cache_key = ?1",
            params![key],
            |row| Ok(CacheEntry {
                content: row.get(0)?,
                model: row.get(1)?,
            }),
        );

        match result {
            Ok(entry) => {
                // Update access time
                let now = chrono::Utc::now().to_rfc3339();
                conn.execute(
                    "UPDATE llm_cache SET accessed_at = ?1 WHERE cache_key = ?2",
                    params![now, key],
                )?;
                Ok(Some(entry))
            }
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Store a response in cache
    pub fn set(&self, key: &str, content: &str, model: &str) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let conn = self.db.conn.lock();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            r#"INSERT INTO llm_cache (cache_key, content, model, created_at, accessed_at)
               VALUES (?1, ?2, ?3, ?4, ?4)
               ON CONFLICT(cache_key) DO UPDATE SET
                   content = excluded.content,
                   model = excluded.model,
                   accessed_at = excluded.accessed_at"#,
            params![key, content, model, now],
        )?;

        Ok(())
    }

    /// Clear old cache entries
    pub fn clear_old(&self, days: u32) -> Result<usize> {
        let conn = self.db.conn.lock();
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);

        let count = conn.execute(
            "DELETE FROM llm_cache WHERE accessed_at < ?1",
            params![cutoff.to_rfc3339()],
        )?;

        Ok(count)
    }
}
