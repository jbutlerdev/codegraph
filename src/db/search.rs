//! Search operations using FTS5

use crate::db::Database;
use crate::error::{Error, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};

/// Search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub path: String,
    pub knowledge_id: String,
    pub score: f64,
    pub purpose: Option<String>,
    pub summary: Option<String>,
}

/// Search parameters
#[derive(Debug, Clone)]
pub struct SearchParams {
    pub query: String,
    pub knowledge_id: Option<String>,
    pub path_prefix: Option<String>,
    pub exclude_suffixes: Vec<String>,
    pub exclude_contains: Vec<String>,
    pub limit: i32,
}

/// Channel for search
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SearchChannel {
    Purpose,
    BusinessContext,
    Paths,
    Keywords,
    Classes,
    Functions,
    ImportsInternal,
    ImportsExternal,
}

impl std::fmt::Display for SearchChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchChannel::Purpose => write!(f, "purpose"),
            SearchChannel::BusinessContext => write!(f, "businessContext"),
            SearchChannel::Paths => write!(f, "paths"),
            SearchChannel::Keywords => write!(f, "keywords"),
            SearchChannel::Classes => write!(f, "classes"),
            SearchChannel::Functions => write!(f, "functions"),
            SearchChannel::ImportsInternal => write!(f, "importsInternal"),
            SearchChannel::ImportsExternal => write!(f, "importsExternal"),
        }
    }
}

/// Search result with channel information
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelSearchResult {
    pub channel: SearchChannel,
    pub hits: Vec<SearchHit>,
}

/// Fused search result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FusedSearchResult {
    pub path: String,
    pub knowledge_id: String,
    pub score: f64,
    pub matched_channels: Vec<String>,
    pub repo_name: String,
}

impl Database {
    /// Search by purpose/summary (FTS)
    pub fn search_purpose(&self, params: &SearchParams) -> Result<Vec<SearchHit>> {
        let conn = self.conn.lock();

        let fts_query = build_fts_query(&params.query);
        let exclude_filter = build_exclude_filter(params);

        let sql = format!(
            r#"SELECT f.relative_path, f.knowledge_id, rank,
                      f.purpose, f.summary
               FROM files f
               JOIN file_purpose_fts fts ON f.id = fts.rowid
               WHERE file_purpose_fts MATCH $1
                 AND ($2 IS NULL OR f.knowledge_id = $2)
                 AND ($3 IS NULL OR f.relative_path LIKE $3 || '%')
                 {}
               ORDER BY rank
               LIMIT $4"#,
            exclude_filter
        );

        let path_prefix = params.path_prefix.as_deref();

        let mut stmt = conn.prepare(&sql).map_err(Error::Database)?;
        let rows = stmt.query_map(
            params![fts_query, params.knowledge_id, path_prefix, params.limit],
            |row| {
                Ok(SearchHit {
                    path: row.get(0)?,
                    knowledge_id: row.get(1)?,
                    score: row.get::<_, f64>(2)?,
                    purpose: row.get(3)?,
                    summary: row.get(4)?,
                })
            },
        ).map_err(Error::Database)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(Error::Database)?);
        }
        Ok(results)
    }

    /// Search by business context (FTS)
    pub fn search_business_context(&self, params: &SearchParams) -> Result<Vec<SearchHit>> {
        let conn = self.conn.lock();

        let fts_query = build_fts_query(&params.query);
        let exclude_filter = build_exclude_filter(params);
        let path_prefix = params.path_prefix.as_deref();

        let sql = format!(
            r#"SELECT f.relative_path, f.knowledge_id, rank,
                      f.purpose, f.summary
               FROM files f
               JOIN file_context_fts fts ON f.id = fts.rowid
               WHERE file_context_fts MATCH $1
                 AND ($2 IS NULL OR f.knowledge_id = $2)
                 AND ($3 IS NULL OR f.relative_path LIKE $3 || '%')
                 {}
               ORDER BY rank
               LIMIT $4"#,
            exclude_filter
        );

        let mut stmt = conn.prepare(&sql).map_err(Error::Database)?;
        let rows = stmt.query_map(
            params![fts_query, params.knowledge_id, path_prefix, params.limit],
            |row| {
                Ok(SearchHit {
                    path: row.get(0)?,
                    knowledge_id: row.get(1)?,
                    score: row.get::<_, f64>(2)?,
                    purpose: row.get(3)?,
                    summary: row.get(4)?,
                })
            },
        ).map_err(Error::Database)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(Error::Database)?);
        }
        Ok(results)
    }

    /// Search by file path (case-insensitive contains)
    pub fn search_paths(&self, params: &SearchParams) -> Result<Vec<SearchHit>> {
        let conn = self.conn.lock();

        let exclude_filter = build_exclude_filter(params);
        let path_prefix = params.path_prefix.as_deref();
        let lower_query = params.query.to_lowercase();

        let sql = format!(
            r#"SELECT f.relative_path, f.knowledge_id, 1.0 as rank,
                      f.purpose, f.summary
               FROM files f
               WHERE LOWER(f.relative_path) LIKE '%' || $1 || '%'
                 AND ($2 IS NULL OR f.knowledge_id = $2)
                 AND ($3 IS NULL OR f.relative_path LIKE $3 || '%')
                 {}
               ORDER BY f.relative_path
               LIMIT $4"#,
            exclude_filter
        );

        let mut stmt = conn.prepare(&sql).map_err(Error::Database)?;
        let rows = stmt.query_map(
            params![lower_query, params.knowledge_id, path_prefix, params.limit],
            |row| {
                Ok(SearchHit {
                    path: row.get(0)?,
                    knowledge_id: row.get(1)?,
                    score: row.get::<_, f64>(2)?,
                    purpose: row.get(3)?,
                    summary: row.get(4)?,
                })
            },
        ).map_err(Error::Database)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(Error::Database)?);
        }
        Ok(results)
    }

    /// Search by keywords (via FTS on keywords table)
    pub fn search_keywords(&self, params: &SearchParams) -> Result<Vec<SearchHit>> {
        let conn = self.conn.lock();

        let fts_query = build_fts_query(&params.query);
        let exclude_filter = build_exclude_filter(params);
        let path_prefix = params.path_prefix.as_deref();

        let sql = format!(
            r#"SELECT f.relative_path, f.knowledge_id, MAX(rank) as score,
                      f.purpose, f.summary
               FROM files f
               JOIN file_keywords fk ON f.id = fk.file_id
               JOIN keywords k ON fk.keyword_id = k.id
               JOIN keyword_name_fts fts ON k.id = fts.rowid
               WHERE keyword_name_fts MATCH $1
                 AND ($2 IS NULL OR f.knowledge_id = $2)
                 AND ($3 IS NULL OR f.relative_path LIKE $3 || '%')
                 {}
               GROUP BY f.id
               ORDER BY score DESC
               LIMIT $4"#,
            exclude_filter
        );

        let mut stmt = conn.prepare(&sql).map_err(Error::Database)?;
        let rows = stmt.query_map(
            params![fts_query, params.knowledge_id, path_prefix, params.limit],
            |row| {
                Ok(SearchHit {
                    path: row.get(0)?,
                    knowledge_id: row.get(1)?,
                    score: row.get::<_, f64>(2)?,
                    purpose: row.get(3)?,
                    summary: row.get(4)?,
                })
            },
        ).map_err(Error::Database)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(Error::Database)?);
        }
        Ok(results)
    }

    /// Search by class names
    pub fn search_classes(&self, params: &SearchParams) -> Result<Vec<SearchHit>> {
        let conn = self.conn.lock();

        let _fts_query = build_fts_query(&params.query);
        let exclude_filter = build_exclude_filter(params);
        let path_prefix = params.path_prefix.as_deref();

        let sql = format!(
            r#"SELECT f.relative_path, f.knowledge_id, 1.0 as score,
                      f.purpose, f.summary
               FROM files f
               JOIN file_classes fc ON f.id = fc.file_id
               JOIN classes c ON fc.class_id = c.id
               WHERE LOWER(c.signature) LIKE '%' || $1 || '%'
                 AND ($2 IS NULL OR f.knowledge_id = $2)
                 AND ($3 IS NULL OR f.relative_path LIKE $3 || '%')
                 {}
               ORDER BY f.relative_path
               LIMIT $4"#,
            exclude_filter
        );

        let mut stmt = conn.prepare(&sql).map_err(Error::Database)?;
        let rows = stmt.query_map(
            params![params.query.to_lowercase(), params.knowledge_id, path_prefix, params.limit],
            |row| {
                Ok(SearchHit {
                    path: row.get(0)?,
                    knowledge_id: row.get(1)?,
                    score: row.get::<_, f64>(2)?,
                    purpose: row.get(3)?,
                    summary: row.get(4)?,
                })
            },
        ).map_err(Error::Database)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(Error::Database)?);
        }
        Ok(results)
    }

    /// Search by function names
    pub fn search_functions(&self, params: &SearchParams) -> Result<Vec<SearchHit>> {
        let conn = self.conn.lock();

        let _fts_query = build_fts_query(&params.query);
        let exclude_filter = build_exclude_filter(params);
        let path_prefix = params.path_prefix.as_deref();

        let sql = format!(
            r#"SELECT f.relative_path, f.knowledge_id, 1.0 as score,
                      f.purpose, f.summary
               FROM files f
               JOIN file_functions ff ON f.id = ff.file_id
               JOIN functions fn ON ff.function_id = fn.id
               WHERE LOWER(fn.signature) LIKE '%' || $1 || '%'
                 AND ($2 IS NULL OR f.knowledge_id = $2)
                 AND ($3 IS NULL OR f.relative_path LIKE $3 || '%')
                 {}
               ORDER BY f.relative_path
               LIMIT $4"#,
            exclude_filter
        );

        let mut stmt = conn.prepare(&sql).map_err(Error::Database)?;
        let rows = stmt.query_map(
            params![params.query.to_lowercase(), params.knowledge_id, path_prefix, params.limit],
            |row| {
                Ok(SearchHit {
                    path: row.get(0)?,
                    knowledge_id: row.get(1)?,
                    score: row.get::<_, f64>(2)?,
                    purpose: row.get(3)?,
                    summary: row.get(4)?,
                })
            },
        ).map_err(Error::Database)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(Error::Database)?);
        }
        Ok(results)
    }

    /// Full multi-channel search
    pub fn smart_search(&self, params: &SearchParams) -> Result<Vec<ChannelSearchResult>> {
        let mut results = Vec::new();

        results.push(ChannelSearchResult {
            channel: SearchChannel::Purpose,
            hits: self.search_purpose(params)?,
        });

        results.push(ChannelSearchResult {
            channel: SearchChannel::BusinessContext,
            hits: self.search_business_context(params)?,
        });

        results.push(ChannelSearchResult {
            channel: SearchChannel::Paths,
            hits: self.search_paths(params)?,
        });

        results.push(ChannelSearchResult {
            channel: SearchChannel::Keywords,
            hits: self.search_keywords(params)?,
        });

        results.push(ChannelSearchResult {
            channel: SearchChannel::Classes,
            hits: Vec::new(),
        });

        results.push(ChannelSearchResult {
            channel: SearchChannel::Functions,
            hits: Vec::new(),
        });

        results.push(ChannelSearchResult {
            channel: SearchChannel::ImportsInternal,
            hits: Vec::new(),
        });

        results.push(ChannelSearchResult {
            channel: SearchChannel::ImportsExternal,
            hits: Vec::new(),
        });

        Ok(results)
    }
}

/// Build FTS5 query from user query
fn build_fts_query(query: &str) -> String {
    let terms: Vec<String> = query
        .split_whitespace()
        .map(|t| {
            let term = t.to_lowercase();
            // Terms with special FTS characters need to be quoted
            if term.contains('-') || term.contains('"') || term.contains(':') {
                // Escape double quotes and wrap in quotes
                let escaped = term.replace('"', "\\\"");
                format!("\"{}\" *", escaped)
            } else {
                format!("{} *", term)
            }
        })
        .collect();
    terms.join(" ")
}


/// Build exclusion filter SQL fragment
fn build_exclude_filter(params: &SearchParams) -> String {
    let mut conditions = Vec::new();

    for suffix in &params.exclude_suffixes {
        conditions.push(format!("f.relative_path NOT LIKE '%{}'", suffix));
    }

    for contains in &params.exclude_contains {
        conditions.push(format!("f.relative_path NOT LIKE '%{}%'", contains));
    }

    if conditions.is_empty() {
        String::new()
    } else {
        format!(" AND {}", conditions.join(" AND "))
    }
}

/// Fuse multiple channel results with weights
pub fn fuse_results(results: Vec<ChannelSearchResult>, weights: &[(&str, f64)]) -> Vec<FusedSearchResult> {
    let mut fused: std::collections::HashMap<String, FusedSearchResult> = std::collections::HashMap::new();

    for result in &results {
        let weight = weights.iter()
            .find(|(name, _)| *name == result.channel.to_string())
            .map(|(_, w)| *w)
            .unwrap_or(1.0);

        for hit in &result.hits {
            let key = format!("{}:{}", hit.knowledge_id, hit.path);
            let entry = fused.entry(key).or_insert_with(|| FusedSearchResult {
                path: hit.path.clone(),
                knowledge_id: hit.knowledge_id.clone(),
                score: 0.0,
                matched_channels: Vec::new(),
                repo_name: String::new(),
            });
            entry.score += hit.score * weight;
            entry.matched_channels.push(result.channel.to_string());
        }
    }

    let mut sorted: Vec<_> = fused.into_values().collect();
    sorted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    sorted
}
