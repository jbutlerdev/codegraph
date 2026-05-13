//! Entity (Keyword, Class, Function, Module) CRUD operations

use crate::db::Database;
use crate::db::files::LinkType;
use crate::error::{Error, Result};
use rusqlite::params;
use serde::{Deserialize, Serialize};

/// Keyword entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyword {
    pub id: i64,
    pub name: String,
}

/// Class entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Class {
    pub id: i64,
    pub signature: String,
}

/// Function entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Function {
    pub id: i64,
    pub signature: String,
}

/// Module entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    pub id: i64,
    pub name: String,
    pub is_external: bool,
}

/// Entity reference result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityReference {
    pub file_path: String,
    pub knowledge_id: String,
    pub link_type: String,
}

/// File dependencies result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDependencies {
    pub classes_defined: Vec<String>,
    pub functions_defined: Vec<String>,
    pub modules_defined: Vec<String>,
    pub modules_imported: Vec<String>,
}

impl Database {
    /// Get or create a keyword
    pub fn get_or_create_keyword(&self, name: &str) -> Result<i64> {
        let conn = self.conn.lock();
        let normalized = name.to_lowercase();

        // Try to get existing
        let existing: Option<i64> = match conn.query_row(
            "SELECT id FROM keywords WHERE name = ?1",
            params![normalized],
            |row| row.get(0),
        ) {
            Ok(id) => Some(id),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(Error::Database(e)),
        };

        if let Some(id) = existing {
            return Ok(id);
        }

        // Insert new
        conn.execute(
            "INSERT INTO keywords (name) VALUES (?1)",
            params![normalized],
        ).map_err(Error::Database)?;

        Ok(conn.last_insert_rowid())
    }

    /// Get or create a class
    pub fn get_or_create_class(&self, signature: &str) -> Result<i64> {
        let conn = self.conn.lock();

        let existing: Option<i64> = match conn.query_row(
            "SELECT id FROM classes WHERE signature = ?1",
            params![signature],
            |row| row.get(0),
        ) {
            Ok(id) => Some(id),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(Error::Database(e)),
        };

        if let Some(id) = existing {
            return Ok(id);
        }

        conn.execute(
            "INSERT INTO classes (signature) VALUES (?1)",
            params![signature],
        ).map_err(Error::Database)?;

        Ok(conn.last_insert_rowid())
    }

    /// Get or create a function
    pub fn get_or_create_function(&self, signature: &str) -> Result<i64> {
        let conn = self.conn.lock();

        let existing: Option<i64> = match conn.query_row(
            "SELECT id FROM functions WHERE signature = ?1",
            params![signature],
            |row| row.get(0),
        ) {
            Ok(id) => Some(id),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(Error::Database(e)),
        };

        if let Some(id) = existing {
            return Ok(id);
        }

        conn.execute(
            "INSERT INTO functions (signature) VALUES (?1)",
            params![signature],
        ).map_err(Error::Database)?;

        Ok(conn.last_insert_rowid())
    }

    /// Get or create a module
    pub fn get_or_create_module(&self, name: &str, is_external: bool) -> Result<i64> {
        let conn = self.conn.lock();

        let existing: Option<i64> = match conn.query_row(
            "SELECT id FROM modules WHERE name = ?1",
            params![name],
            |row| row.get(0),
        ) {
            Ok(id) => Some(id),
            Err(rusqlite::Error::QueryReturnedNoRows) => None,
            Err(e) => return Err(Error::Database(e)),
        };

        if let Some(id) = existing {
            return Ok(id);
        }

        conn.execute(
            "INSERT INTO modules (name, is_external) VALUES (?1, ?2)",
            params![name, is_external],
        ).map_err(Error::Database)?;

        Ok(conn.last_insert_rowid())
    }

    /// Link a file to keywords
    pub fn link_file_keywords(&self, file_id: i64, keyword_ids: &[i64], link_type: LinkType) -> Result<()> {
        if keyword_ids.is_empty() {
            return Ok(());
        }

        let conn = self.conn.lock();
        let lt = link_type.to_string();

        for kid in keyword_ids {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO file_keywords (file_id, keyword_id, link_type) VALUES (?1, ?2, ?3)",
                params![file_id, kid, lt],
            );
        }

        Ok(())
    }

    /// Link a file to classes
    pub fn link_file_classes(&self, file_id: i64, class_ids: &[i64], link_type: LinkType) -> Result<()> {
        if class_ids.is_empty() {
            return Ok(());
        }

        let conn = self.conn.lock();
        let lt = link_type.to_string();

        for cid in class_ids {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO file_classes (file_id, class_id, link_type) VALUES (?1, ?2, ?3)",
                params![file_id, cid, lt],
            );
        }

        Ok(())
    }

    /// Link a file to functions
    pub fn link_file_functions(&self, file_id: i64, function_ids: &[i64], link_type: LinkType) -> Result<()> {
        if function_ids.is_empty() {
            return Ok(());
        }

        let conn = self.conn.lock();
        let lt = link_type.to_string();

        for fid in function_ids {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO file_functions (file_id, function_id, link_type) VALUES (?1, ?2, ?3)",
                params![file_id, fid, lt],
            );
        }

        Ok(())
    }

    /// Link a file to internal modules (defined or imported)
    pub fn link_file_imports_internal(&self, file_id: i64, module_ids: &[i64], link_type: LinkType) -> Result<()> {
        if module_ids.is_empty() {
            return Ok(());
        }

        let conn = self.conn.lock();
        let lt = link_type.to_string();

        for mid in module_ids {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO file_imports_internal (file_id, module_id, link_type) VALUES (?1, ?2, ?3)",
                params![file_id, mid, lt],
            );
        }

        Ok(())
    }

    /// Link a file to external imports
    pub fn link_file_imports_external(&self, file_id: i64, module_ids: &[i64], link_type: LinkType) -> Result<()> {
        if module_ids.is_empty() {
            return Ok(());
        }

        let conn = self.conn.lock();
        let lt = link_type.to_string();

        for mid in module_ids {
            let _ = conn.execute(
                "INSERT OR IGNORE INTO file_imports_external (file_id, module_id, link_type) VALUES (?1, ?2, ?3)",
                params![file_id, mid, lt],
            );
        }

        Ok(())
    }

    /// Clear file links (before re-linking)
    pub fn clear_file_links(&self, file_id: i64) -> Result<()> {
        let conn = self.conn.lock();

        let _ = conn.execute("DELETE FROM file_keywords WHERE file_id = ?1", params![file_id]);
        let _ = conn.execute("DELETE FROM file_classes WHERE file_id = ?1", params![file_id]);
        let _ = conn.execute("DELETE FROM file_functions WHERE file_id = ?1", params![file_id]);
        let _ = conn.execute("DELETE FROM file_imports_internal WHERE file_id = ?1", params![file_id]);
        let _ = conn.execute("DELETE FROM file_imports_external WHERE file_id = ?1", params![file_id]);

        Ok(())
    }

    /// Search keywords by name (FTS)
    pub fn search_keyword_entities(&self, query: &str, limit: i32) -> Result<Vec<Keyword>> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            r#"SELECT k.id, k.name FROM keywords k
               JOIN keyword_name_fts fts ON k.id = fts.rowid
               WHERE keyword_name_fts MATCH ?1
               ORDER BY rank
               LIMIT ?2"#
        ).map_err(Error::Database)?;

        let fts_query = format!("{}*", query.to_lowercase());

        let rows = stmt.query_map(params![fts_query, limit], |row| {
            Ok(Keyword {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        }).map_err(Error::Database)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(Error::Database)?);
        }
        Ok(results)
    }

    /// Search symbols (classes + functions) by signature (FTS)
    pub fn search_symbols(&self, query: &str, label: Option<&str>, limit: i32) -> Result<Vec<(String, String)>> {
        let conn = self.conn.lock();

        let table = match label {
            Some("class") => "classes",
            Some("function") => "functions",
            _ => return Err(Error::InvalidInput("label must be 'class' or 'function'".to_string())),
        };

        let sql = format!(
            r#"SELECT s.signature FROM {} s
               JOIN symbol_fts fts ON s.id = fts.rowid
               WHERE symbol_fts MATCH ?1
               ORDER BY rank
               LIMIT ?2"#,
            table
        );

        let mut stmt = conn.prepare(&sql).map_err(Error::Database)?;
        let fts_query = format!("{}*", query.to_lowercase());

        let rows = stmt.query_map(params![fts_query, limit], |row| {
            Ok((row.get::<_, String>(0)?, label.unwrap().to_string()))
        }).map_err(Error::Database)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(Error::Database)?);
        }
        Ok(results)
    }

    /// Get keywords for a file
    pub fn get_file_keywords(&self, file_id: i64) -> Result<Vec<String>> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            "SELECT k.name FROM keywords k JOIN file_keywords fk ON k.id = fk.keyword_id WHERE fk.file_id = ?1"
        ).map_err(Error::Database)?;

        let rows = stmt.query_map(params![file_id], |row| row.get(0))
            .map_err(Error::Database)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(Error::Database)?);
        }
        Ok(results)
    }

    /// Get classes for a file
    pub fn get_file_classes(&self, file_id: i64) -> Result<Vec<String>> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            "SELECT c.signature FROM classes c JOIN file_classes fc ON c.id = fc.class_id WHERE fc.file_id = ?1"
        ).map_err(Error::Database)?;

        let rows = stmt.query_map(params![file_id], |row| row.get(0))
            .map_err(Error::Database)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(Error::Database)?);
        }
        Ok(results)
    }

    /// Get functions for a file
    pub fn get_file_functions(&self, file_id: i64) -> Result<Vec<String>> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            "SELECT fn.signature FROM functions fn JOIN file_functions ff ON fn.id = ff.function_id WHERE ff.file_id = ?1"
        ).map_err(Error::Database)?;

        let rows = stmt.query_map(params![file_id], |row| row.get(0))
            .map_err(Error::Database)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(Error::Database)?);
        }
        Ok(results)
    }

    /// Get imports for a file
    pub fn get_file_imports(&self, file_id: i64) -> Result<(Vec<String>, Vec<String>)> {
        let conn = self.conn.lock();

        let mut internal_stmt = conn.prepare(
            "SELECT m.name FROM modules m JOIN file_imports_internal fi ON m.id = fi.module_id WHERE fi.file_id = ?1"
        ).map_err(Error::Database)?;
        
        let internal_rows = internal_stmt.query_map(params![file_id], |row| row.get::<_, String>(0))
            .map_err(Error::Database)?;
        let internal: Vec<String> = internal_rows.filter_map(|r| r.ok()).collect();

        let mut external_stmt = conn.prepare(
            "SELECT m.name FROM modules m JOIN file_imports_external fe ON m.id = fe.module_id WHERE fe.file_id = ?1"
        ).map_err(Error::Database)?;
        
        let external_rows = external_stmt.query_map(params![file_id], |row| row.get::<_, String>(0))
            .map_err(Error::Database)?;
        let external: Vec<String> = external_rows.filter_map(|r| r.ok()).collect();

        Ok((internal, external))
    }

    /// Find where an entity is defined and what files reference it
    pub fn get_entity_references(&self, entity_type: &str, entity_name: &str, link_type: Option<LinkType>, knowledge_id: Option<&str>) -> Result<Vec<EntityReference>> {
        let conn = self.conn.lock();
        
        let (table, id_col) = match entity_type {
            "class" => ("classes", "class_id"),
            "function" => ("functions", "function_id"),
            "module" => ("modules", "module_id"),
            _ => return Err(Error::InvalidInput("entity_type must be 'class', 'function', or 'module'".to_string())),
        };

        let link_table = match entity_type {
            "class" => "file_classes",
            "function" => "file_functions",
            "module" => "file_imports_internal",
            _ => return Err(Error::InvalidInput("entity_type must be 'class', 'function', or 'module'".to_string())),
        };

        let link_filter = match link_type {
            Some(lt) => format!("AND e.link_type = '{}'", lt),
            None => String::new(),
        };

        let knowledge_filter = match knowledge_id {
            Some(kid) => format!("AND f.knowledge_id = '{}'", kid),
            None => String::new(),
        };

        // For classes/functions, match if the name appears at start (before the colon or parens)
        // For modules, exact match
        let (sql_extra, lookup_value) = match entity_type {
            "class" | "function" => {
                // Match signatures where the name part (before : or ( ) starts with the search term
                let like_pattern = format!("{}%", entity_name.to_lowercase());
                (
                    format!(
                        r#"AND LOWER(t.{}) LIKE ?1"#,
                        if entity_type == "module" { "name" } else { "signature" }
                    ),
                    like_pattern,
                )
            }
            "module" => {
                (
                    format!(
                        r#"AND LOWER(t.name) = LOWER(?1)"#
                    ),
                    entity_name.to_string(),
                )
            }
            _ => (String::new(), entity_name.to_string()),
        };

        let sql = format!(
            r#"SELECT f.relative_path, f.knowledge_id, e.link_type
               FROM files f
               JOIN {} e ON f.id = e.file_id
               JOIN {} t ON e.{} = t.id
               WHERE 1=1
               {}
               {}
               {}"#,
            link_table, table, id_col,
            sql_extra,
            link_filter, knowledge_filter
        );

        let mut stmt = conn.prepare(&sql).map_err(Error::Database)?;
        let rows = stmt.query_map(params![lookup_value], |row| {
            Ok(EntityReference {
                file_path: row.get(0)?,
                knowledge_id: row.get(1)?,
                link_type: row.get(2)?,
            })
        }).map_err(Error::Database)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(Error::Database)?);
        }
        Ok(results)
    }

    /// Get the file that defines an entity
    pub fn get_entity_definition(&self, entity_type: &str, entity_name: &str, knowledge_id: Option<&str>) -> Result<Option<EntityReference>> {
        let refs = self.get_entity_references(entity_type, entity_name, Some(LinkType::Defines), knowledge_id)?;
        Ok(refs.into_iter().next())
    }

    /// Get all files that reference an entity (imports/uses)
    pub fn get_entity_usages(&self, entity_type: &str, entity_name: &str, knowledge_id: Option<&str>) -> Result<Vec<EntityReference>> {
        self.get_entity_references(entity_type, entity_name, Some(LinkType::References), knowledge_id)
    }

    /// Get files that define and files that use an entity
    pub fn get_entity_complete_references(&self, entity_type: &str, entity_name: &str, knowledge_id: Option<&str>) -> Result<(Option<EntityReference>, Vec<EntityReference>)> {
        let refs = self.get_entity_references(entity_type, entity_name, None, knowledge_id)?;
        
        let definition = refs.iter().find(|r| r.link_type == "defines").cloned();
        let usages: Vec<EntityReference> = refs.into_iter().filter(|r| r.link_type == "references").collect();
        
        Ok((definition, usages))
    }

    /// Find files that share entities (similar dependencies)
    pub fn get_similar_files(&self, file_id: i64, knowledge_id: Option<&str>, limit: i32) -> Result<Vec<(String, String, i32)>> {
        let conn = self.conn.lock();

        let knowledge_filter = match knowledge_id {
            Some(kid) => format!("AND f2.knowledge_id = '{}'", kid),
            None => String::new(),
        };

        let sql = format!(
            r#"SELECT f2.relative_path, f2.knowledge_id, COUNT(*) as shared_count
               FROM (
                   SELECT class_id FROM file_classes WHERE file_id = ?1
                   UNION ALL
                   SELECT function_id FROM file_functions WHERE file_id = ?1
                   UNION ALL  
                   SELECT module_id FROM file_imports_internal WHERE file_id = ?1
               ) entities
               JOIN file_classes fc ON entities.class_id = fc.class_id AND fc.file_id != ?1
               JOIN files f2 ON fc.file_id = f2.id
               {}
               GROUP BY f2.relative_path, f2.knowledge_id
               ORDER BY shared_count DESC
               LIMIT ?2"#,
            knowledge_filter
        );

        let mut stmt = conn.prepare(&sql).map_err(Error::Database)?;
        let rows = stmt.query_map(params![file_id, limit], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i32>(2)?))
        }).map_err(Error::Database)?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(Error::Database)?);
        }
        Ok(results)
    }

    /// Get dependencies of a file (entities it defines and imports)
    pub fn get_file_dependencies(&self, file_id: i64) -> Result<FileDependencies> {
        let conn = self.conn.lock();

        // Classes defined
        let mut class_stmt = conn.prepare(
            "SELECT c.signature FROM classes c JOIN file_classes fc ON c.id = fc.class_id WHERE fc.file_id = ?1 AND fc.link_type = 'defines'"
        ).map_err(Error::Database)?;
        let classes: Vec<String> = class_stmt.query_map(params![file_id], |r| r.get(0))
            .map_err(Error::Database)?
            .filter_map(|r| r.ok())
            .collect();

        // Functions defined
        let mut func_stmt = conn.prepare(
            "SELECT fn.signature FROM functions fn JOIN file_functions ff ON fn.id = ff.function_id WHERE ff.file_id = ?1 AND ff.link_type = 'defines'"
        ).map_err(Error::Database)?;
        let functions: Vec<String> = func_stmt.query_map(params![file_id], |r| r.get(0))
            .map_err(Error::Database)?
            .filter_map(|r| r.ok())
            .collect();

        // Modules (defined and imported)
        let mut mod_stmt = conn.prepare(
            "SELECT m.name, fi.link_type FROM modules m JOIN file_imports_internal fi ON m.id = fi.module_id WHERE fi.file_id = ?1"
        ).map_err(Error::Database)?;
        let module_rows: Vec<(String, String)> = mod_stmt.query_map(params![file_id], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(Error::Database)?
            .filter_map(|r| r.ok())
            .collect();
        
        let modules_defined: Vec<String> = module_rows.iter().filter(|(_, lt)| lt == "defines").map(|(n, _)| n.clone()).collect();
        let modules_imported: Vec<String> = module_rows.iter().filter(|(_, lt)| lt == "references").map(|(n, _)| n.clone()).collect();

        Ok(FileDependencies {
            classes_defined: classes,
            functions_defined: functions,
            modules_defined,
            modules_imported,
        })
    }

    /// Get reverse dependencies (files that depend on this file)
    pub fn get_file_dependents(&self, file_id: i64, knowledge_id: Option<&str>, limit: i32) -> Result<Vec<(String, String)>> {
        let conn = self.conn.lock();

        // First get what this file defines
        let mut defined_stmt = conn.prepare(
            r#"SELECT 'class', class_id FROM file_classes WHERE file_id = ?1 AND link_type = 'defines'
               UNION ALL
               SELECT 'function', function_id FROM file_functions WHERE file_id = ?1 AND link_type = 'defines'
               UNION ALL
               SELECT 'module', module_id FROM file_imports_internal WHERE file_id = ?1 AND link_type = 'defines'"#
        ).map_err(Error::Database)?;

        let defined_entities: Vec<(String, i64)> = defined_stmt.query_map(params![file_id], |r| {
            Ok((r.get(0)?, r.get(1)?))
        }).map_err(Error::Database)?
            .filter_map(|r| r.ok())
            .collect();

        if defined_entities.is_empty() {
            return Ok(Vec::new());
        }

        let knowledge_filter = match knowledge_id {
            Some(kid) => format!("AND f.knowledge_id = '{}'", kid),
            None => String::new(),
        };

        let mut results: Vec<(String, String)> = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for (entity_type, entity_id) in defined_entities {
            let (table, id_column) = match entity_type.as_str() {
                "class" => ("file_classes", "class_id"),
                "function" => ("file_functions", "function_id"),
                "module" => ("file_imports_internal", "module_id"),
                _ => continue,
            };

            let sql = format!(
                r#"SELECT f.relative_path, f.knowledge_id
                   FROM files f
                   JOIN {} e ON f.id = e.file_id
                   WHERE e.{} = ?1 AND e.file_id != ?2 AND e.link_type = 'references'
                   {}"#,
                table, id_column, knowledge_filter
            );

            let mut stmt = conn.prepare(&sql).map_err(Error::Database)?;
            let rows = stmt.query_map(params![entity_id, file_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            }).map_err(Error::Database)?;

            for row in rows {
                if let Ok((path, kid)) = row {
                    let key = format!("{}/{}", kid, path);
                    if seen.insert(key) {
                        results.push((path, kid));
                    }
                }
            }
        }

        results.sort();
        results.truncate(limit as usize);
        Ok(results)
    }
}
