//! Database connection management

use anyhow::{Context, Result};
use parking_lot::Mutex;
use rusqlite::Connection;
use std::path::Path;

use crate::config::db_path;

/// Database connection pool (single-threaded SQLite with parking_lot mutex)
pub struct Database {
    pub(crate) conn: Mutex<Connection>,
}

impl Database {
    /// Open or create the database
    pub fn open() -> Result<Self> {
        let path = db_path()?;
        Self::open_path(&path)
    }

    /// Open database at specific path
    pub fn open_path(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .context("Cannot open database")?;

        // Enable foreign keys
        conn.execute("PRAGMA foreign_keys = ON", [])?;

        // Enable WAL mode for better concurrency
        conn.execute_batch("PRAGMA journal_mode = WAL")?;

        let db = Self {
            conn: Mutex::new(conn),
        };

        db.init_schema()?;

        Ok(db)
    }

    /// Get a connection reference
    pub fn conn(&self) -> parking_lot::MutexGuard<'_, Connection> {
        self.conn.lock()
    }

    /// Initialize the database schema
    fn init_schema(&self) -> Result<()> {
        let conn = self.conn.lock();

        conn.execute_batch(r#"
            -- Knowledge (indexed repository)
            CREATE TABLE IF NOT EXISTS knowledge (
                id TEXT PRIMARY KEY,
                repo_name TEXT NOT NULL,
                source_kind TEXT NOT NULL CHECK (source_kind IN ('github', 'local')),
                source_url TEXT,
                source_path TEXT,
                branch TEXT,
                commit_hash TEXT,
                state TEXT NOT NULL DEFAULT 'created' CHECK (state IN ('created', 'queued', 'ingested', 'processing', 'processed', 'failed')),
                file_count INTEGER DEFAULT 0,
                total_files INTEGER DEFAULT 0,
                processed_files INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- Files (per-file metadata)
            CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                knowledge_id TEXT NOT NULL REFERENCES knowledge(id) ON DELETE CASCADE,
                relative_path TEXT NOT NULL,
                language TEXT,
                sha256 TEXT NOT NULL,
                size_bytes INTEGER,
                purpose TEXT,
                summary TEXT,
                business_context TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(knowledge_id, relative_path)
            );

            -- Keywords (deduplicated across graph)
            CREATE TABLE IF NOT EXISTS keywords (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE
            );

            -- Classes (deduplicated)
            CREATE TABLE IF NOT EXISTS classes (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                signature TEXT NOT NULL UNIQUE
            );

            -- Functions (deduplicated)
            CREATE TABLE IF NOT EXISTS functions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                signature TEXT NOT NULL UNIQUE
            );

            -- Modules (deduplicated)
            CREATE TABLE IF NOT EXISTS modules (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                is_external BOOLEAN NOT NULL DEFAULT FALSE
            );

            -- Edges
            -- Relationship types: 'defines' (entity is defined here) | 'references' (entity is used/imported)
            CREATE TABLE IF NOT EXISTS file_keywords (
                file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                keyword_id INTEGER NOT NULL REFERENCES keywords(id) ON DELETE CASCADE,
                link_type TEXT NOT NULL DEFAULT 'references' CHECK (link_type IN ('defines', 'references')),
                PRIMARY KEY (file_id, keyword_id)
            );

            CREATE TABLE IF NOT EXISTS file_classes (
                file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                class_id INTEGER NOT NULL REFERENCES classes(id) ON DELETE CASCADE,
                link_type TEXT NOT NULL DEFAULT 'references' CHECK (link_type IN ('defines', 'references')),
                PRIMARY KEY (file_id, class_id)
            );

            CREATE TABLE IF NOT EXISTS file_functions (
                file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                function_id INTEGER NOT NULL REFERENCES functions(id) ON DELETE CASCADE,
                link_type TEXT NOT NULL DEFAULT 'references' CHECK (link_type IN ('defines', 'references')),
                PRIMARY KEY (file_id, function_id)
            );

            CREATE TABLE IF NOT EXISTS file_imports_internal (
                file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                module_id INTEGER NOT NULL REFERENCES modules(id) ON DELETE CASCADE,
                link_type TEXT NOT NULL DEFAULT 'defines' CHECK (link_type IN ('defines', 'references')),
                PRIMARY KEY (file_id, module_id)
            );

            CREATE TABLE IF NOT EXISTS file_imports_external (
                file_id INTEGER NOT NULL REFERENCES files(id) ON DELETE CASCADE,
                module_id INTEGER NOT NULL REFERENCES modules(id) ON DELETE CASCADE,
                link_type TEXT NOT NULL DEFAULT 'references' CHECK (link_type IN ('defines', 'references')),
                PRIMARY KEY (file_id, module_id)
            );

            -- File versions (for history)
            CREATE TABLE IF NOT EXISTS file_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                knowledge_id TEXT NOT NULL,
                relative_path TEXT NOT NULL,
                commit_hash TEXT NOT NULL,
                language TEXT,
                sha256 TEXT,
                purpose TEXT,
                summary TEXT,
                business_context TEXT,
                snapshot_at TEXT NOT NULL,
                UNIQUE(knowledge_id, relative_path, commit_hash)
            );

            -- Token usage tracking
            CREATE TABLE IF NOT EXISTS token_usage (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                knowledge_id TEXT NOT NULL,
                model TEXT NOT NULL,
                input_tokens INTEGER NOT NULL,
                output_tokens INTEGER NOT NULL,
                cost_usd REAL,
                created_at TEXT NOT NULL
            );

            -- LLM cache
            CREATE TABLE IF NOT EXISTS llm_cache (
                cache_key TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                model TEXT NOT NULL,
                created_at TEXT NOT NULL,
                accessed_at TEXT NOT NULL
            );
        "#)?;

        // Create indexes
        conn.execute_batch(r#"
            CREATE INDEX IF NOT EXISTS idx_files_knowledge ON files(knowledge_id);
            CREATE INDEX IF NOT EXISTS idx_files_path ON files(relative_path);
            CREATE INDEX IF NOT EXISTS idx_files_sha ON files(sha256);
            CREATE INDEX IF NOT EXISTS idx_file_keywords_file ON file_keywords(file_id);
            CREATE INDEX IF NOT EXISTS idx_file_keywords_keyword ON file_keywords(keyword_id);
            CREATE INDEX IF NOT EXISTS idx_file_classes_file ON file_classes(file_id);
            CREATE INDEX IF NOT EXISTS idx_file_functions_file ON file_functions(file_id);
            CREATE INDEX IF NOT EXISTS idx_token_usage_knowledge ON token_usage(knowledge_id);
        "#)?;

        // Create FTS virtual tables
        conn.execute_batch(r#"
            CREATE VIRTUAL TABLE IF NOT EXISTS file_purpose_fts USING fts5(
                purpose, summary, content='files', content_rowid='id',
                tokenize='porter unicode61'
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS file_context_fts USING fts5(
                business_context, content='files', content_rowid='id',
                tokenize='porter unicode61'
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS keyword_name_fts USING fts5(
                name, content='keywords', content_rowid='id',
                tokenize='porter unicode61'
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS symbol_fts USING fts5(
                signature, content='classes', content_rowid='id',
                tokenize='porter unicode61'
            );
        "#)?;

        // Create triggers to keep FTS in sync
        conn.execute_batch(r#"
            CREATE TRIGGER IF NOT EXISTS files_ai AFTER INSERT ON files BEGIN
                INSERT INTO file_purpose_fts(rowid, purpose, summary) VALUES (new.id, new.purpose, new.summary);
                INSERT INTO file_context_fts(rowid, business_context) VALUES (new.id, new.business_context);
            END;

            CREATE TRIGGER IF NOT EXISTS files_ad AFTER DELETE ON files BEGIN
                INSERT INTO file_purpose_fts(file_purpose_fts, rowid, purpose, summary) VALUES ('delete', old.id, old.purpose, old.summary);
                INSERT INTO file_context_fts(file_context_fts, rowid, business_context) VALUES ('delete', old.id, old.business_context);
            END;

            CREATE TRIGGER IF NOT EXISTS files_au AFTER UPDATE ON files BEGIN
                INSERT INTO file_purpose_fts(file_purpose_fts, rowid, purpose, summary) VALUES ('delete', old.id, old.purpose, old.summary);
                INSERT INTO file_purpose_fts(rowid, purpose, summary) VALUES (new.id, new.purpose, new.summary);
                INSERT INTO file_context_fts(file_context_fts, rowid, business_context) VALUES ('delete', old.id, old.business_context);
                INSERT INTO file_context_fts(rowid, business_context) VALUES (new.id, new.business_context);
            END;

            -- Keywords FTS triggers
            CREATE TRIGGER IF NOT EXISTS keywords_ai AFTER INSERT ON keywords BEGIN
                INSERT INTO keyword_name_fts(rowid, name) VALUES (new.id, new.name);
            END;

            CREATE TRIGGER IF NOT EXISTS keywords_ad AFTER DELETE ON keywords BEGIN
                INSERT INTO keyword_name_fts(keyword_name_fts, rowid, name) VALUES ('delete', old.id, old.name);
            END;

            CREATE TRIGGER IF NOT EXISTS keywords_au AFTER UPDATE ON keywords BEGIN
                INSERT INTO keyword_name_fts(keyword_name_fts, rowid, name) VALUES ('delete', old.id, old.name);
                INSERT INTO keyword_name_fts(rowid, name) VALUES (new.id, new.name);
            END;
        "#)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_open_database() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open_path(&db_path).unwrap();

        // Verify tables exist
        let conn = db.conn();
        let count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table'",
            [],
            |row| row.get(0),
        ).unwrap();

        assert!(count > 0);
    }
}
