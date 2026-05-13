//! CLI command implementations

use crate::config::{load_config, get_config_value, set_config_value};
use crate::cli::commands::ConfigCommands;
use crate::db::{Database, SearchParams};
use anyhow::Result as AnyResult;
use colored::Colorize;

/// List indexed repositories
pub async fn list_repos() -> AnyResult<()> {
    let db = Database::open()?;
    let repos = db.list_knowledge()?;

    if repos.is_empty() {
        println!("{}", "No indexed repositories.".yellow());
        return Ok(());
    }

    println!("\n{} {}\n", "REPOS".bold(), format!("({} total)", repos.len()).dimmed());

    for (i, repo) in repos.iter().enumerate() {
        let state = match repo.state {
            crate::db::KnowledgeState::Processed => "PROCESSED".green(),
            crate::db::KnowledgeState::Processing => "PROCESSING".yellow(),
            crate::db::KnowledgeState::Failed => "FAILED".red(),
            crate::db::KnowledgeState::Queued => "QUEUED".cyan(),
            _ => format!("{}", repo.state).white(),
        };

        let source = match &repo.source {
            crate::db::KnowledgeSource::Github { url, branch } => {
                if let Some(b) = branch {
                    format!("{} ({})", url, b)
                } else {
                    url.clone()
                }
            }
            crate::db::KnowledgeSource::Local { path } => path.clone(),
        };

        let progress = if repo.total_files > 0 {
            format!("{}/{}", repo.processed_files, repo.total_files)
        } else {
            format!("{} files", repo.file_count)
        };

        println!(
            " {}  {}  {}  {}",
            format!("{}", i + 1).dimmed(),
            repo.id.dimmed(),
            repo.repo_name.bold(),
            format!("[{}] {}", state, progress).dimmed()
        );
        println!("    {}\n", source.dimmed());
    }

    Ok(())
}

/// Search indexed code (full-text search across purpose, summary, businessContext)
pub async fn search(query: &str, repo_id: Option<&str>, limit: usize, json: bool) -> AnyResult<()> {
    let db = Database::open()?;

    let params = SearchParams {
        query: query.to_string(),
        knowledge_id: repo_id.map(String::from),
        path_prefix: None,
        exclude_suffixes: Vec::new(),
        exclude_contains: Vec::new(),
        limit: limit as i32,
    };

    // Search across all relevant fields
    let purpose_results = db.search_purpose(&params)?;
    let context_results = db.search_business_context(&params)?;
    let path_results = db.search_paths(&params)?;

    // Merge and dedupe results (simple approach: combine all, dedupe by file path)
    let mut all_results: Vec<_> = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    for r in purpose_results.into_iter().chain(context_results).chain(path_results) {
        if seen_paths.insert(r.path.clone()) {
            all_results.push(r);
        }
    }

    // Sort by score descending
    all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    if json {
        println!("{}", serde_json::to_string_pretty(&all_results)?);
    } else {
        if all_results.is_empty() {
            println!("{}", "No results found.".yellow());
            return Ok(());
        }

        println!("\n{} \"{}\"\n", "SEARCH RESULTS".bold(), query);
        println!("{} results\n", all_results.len());

        for result in all_results.iter().take(limit) {
            println!("  {}  {}", result.path.bold(), format!("(score: {:.2})", result.score).dimmed());
            if let Some(purpose) = &result.purpose {
                println!("    {}", purpose.dimmed());
            }
            println!();
        }
    }

    Ok(())
}

/// Lookup keywords, classes, functions, and modules
pub async fn lookup(term: &str, repo_id: Option<&str>, json: bool) -> AnyResult<()> {
    let db = Database::open()?;

    let params = SearchParams {
        query: term.to_string(),
        knowledge_id: repo_id.map(String::from),
        path_prefix: None,
        exclude_suffixes: Vec::new(),
        exclude_contains: Vec::new(),
        limit: 50,
    };

    let keyword_results = db.search_keywords(&params)?;
    let class_results = db.search_classes(&params)?;
    let function_results = db.search_functions(&params)?;

    #[derive(serde::Serialize)]
    struct LookupResult {
        term: String,
        keywords: Vec<crate::db::SearchHit>,
        classes: Vec<crate::db::SearchHit>,
        functions: Vec<crate::db::SearchHit>,
    }

    if json {
        let result = LookupResult {
            term: term.to_string(),
            keywords: keyword_results,
            classes: class_results,
            functions: function_results,
        };
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let total = keyword_results.len() + class_results.len() + function_results.len();
        if total == 0 {
            println!("{}", "No entities found.".yellow());
            return Ok(());
        }

        println!("\n{} \"{}\"\n", "LOOKUP RESULTS".bold(), term);
        println!("{} entities found\n", total);

        if !keyword_results.is_empty() {
            println!("  {} Keywords\n", "Keywords:".cyan());
            for r in keyword_results.iter().take(10) {
                println!("    - {} ({})", r.path.bold(), format!("score: {:.2}", r.score).dimmed());
            }
            if keyword_results.len() > 10 {
                println!("    ... and {} more", keyword_results.len() - 10);
            }
            println!();
        }

        if !class_results.is_empty() {
            println!("  {} Classes\n", "Classes:".cyan());
            for r in class_results.iter().take(10) {
                println!("    - {} ({})", r.path.bold(), format!("score: {:.2}", r.score).dimmed());
            }
            if class_results.len() > 10 {
                println!("    ... and {} more", class_results.len() - 10);
            }
            println!();
        }

        if !function_results.is_empty() {
            println!("  {} Functions\n", "Functions:".cyan());
            for r in function_results.iter().take(10) {
                println!("    - {} ({})", r.path.bold(), format!("score: {:.2}", r.score).dimmed());
            }
            if function_results.len() > 10 {
                println!("    ... and {} more", function_results.len() - 10);
            }
            println!();
        }
    }

    Ok(())
}

/// Show file metadata and optionally content
pub async fn cat_file(
    repo_id: &str, 
    file_path: &str, 
    show_content: bool,
    show_numbers: bool, 
    range: Option<&str>,
    search_term: Option<&str>,
) -> AnyResult<()> {
    let db = Database::open()?;
    
    // Resolve repo_id (could be UUID, path, or ".")
    let resolved_repo_id = if db.get_knowledge(repo_id)?.is_some() {
        repo_id.to_string()
    } else if let Some(repo) = db.get_knowledge_by_path(repo_id)? {
        repo.id.clone()
    } else {
        repo_id.to_string()
    };
    
    let file = db.get_file(&resolved_repo_id, file_path)?;
    
    match file {
        Some(f) => {
            // Show metadata header
            println!("{}{}\n", "FILE".bold(), file_path);
            println!("  {:20} {}", "Repository".dimmed(), resolved_repo_id);
            if let Some(lang) = &f.language {
                println!("  {:20} {}", "Language".dimmed(), lang);
            }
            println!("  {:20} {}", "Size".dimmed(), format!("{} bytes", f.size_bytes));
            println!("  {:20} {}", "SHA256".dimmed(), f.sha256);
            
            if let Some(purpose) = &f.purpose {
                println!("\n  {} {}\n", "Purpose:".cyan(), purpose);
            }
            
            if let Some(summary) = &f.summary {
                println!("  {} {}\n", "Summary:".cyan(), summary);
            }
            
            if let Some(ctx) = &f.business_context {
                println!("  {} {}\n", "Business Context:".cyan(), ctx);
            }

            // Get entities
            let keywords = db.get_file_keywords(f.id)?;
            let classes = db.get_file_classes(f.id)?;
            let functions = db.get_file_functions(f.id)?;
            let (internal_imports, external_imports) = db.get_file_imports(f.id)?;

            if !keywords.is_empty() {
                println!("  {} {}", "Keywords:".cyan(), keywords.join(", "));
            }
            if !classes.is_empty() {
                println!("  {} {}", "Classes:".cyan(), classes.join("; "));
            }
            if !functions.is_empty() {
                println!("  {} {}", "Functions:".cyan(), functions.join("; "));
            }
            if !internal_imports.is_empty() {
                println!("  {} {}", "Internal Imports:".cyan(), internal_imports.join(", "));
            }
            if !external_imports.is_empty() {
                println!("  {} {}", "External Imports:".cyan(), external_imports.join(", "));
            }

            // Show content if requested
            if show_content {
                // For local repos, use source_path; for github repos, use repos_dir
                let repo_path = if let Some(repo) = db.get_knowledge(repo_id)? {
                    match &repo.source {
                        crate::db::KnowledgeSource::Local { path } => std::path::PathBuf::from(path),
                        crate::db::KnowledgeSource::Github { .. } => crate::config::repos_dir()?.join(repo_id),
                    }
                } else if let Some(repo) = db.get_knowledge_by_path(repo_id)? {
                    // Found by path (local repos)
                    match &repo.source {
                        crate::db::KnowledgeSource::Local { path } => std::path::PathBuf::from(path),
                        crate::db::KnowledgeSource::Github { .. } => crate::config::repos_dir()?.join(&repo.id),
                    }
                } else {
                    // Not in DB - try as a direct path
                    let path = std::path::PathBuf::from(repo_id);
                    if path.exists() {
                        path
                    } else {
                        // Fall back to repos_dir (for github repos)
                        crate::config::repos_dir()?.join(repo_id)
                    }
                };
                let full_path = repo_path.join(file_path);

                if full_path.exists() {
                    let content = std::fs::read_to_string(&full_path)?;
                    let lines: Vec<&str> = content.lines().collect();
                    let total_lines = lines.len();

                    // Determine line range
                    let (start, end) = if let Some(range_str) = range {
                        parse_range(range_str, total_lines)?
                    } else {
                        (0, total_lines)
                    };

                    // Search within content if requested
                    if let Some(search) = search_term {
                        let search_lower = search.to_lowercase();
                        let context = 3; // Lines of context
                        
                        println!("\n{} {}", "SEARCH".bold(), format!("\"{}\"", search).yellow());
                        println!("  {} matches in {} lines\n", file_path, total_lines);
                        
                        let mut matches = Vec::new();
                        for (i, line) in lines.iter().enumerate() {
                            if line.to_lowercase().contains(&search_lower) {
                                matches.push(i);
                            }
                        }

                        if matches.is_empty() {
                            println!("  {}", "No matches found.".dimmed());
                        } else {
                            for (match_idx, match_line) in matches.iter().enumerate() {
                                let context_start = match_line.saturating_sub(context);
                                let context_end = (*match_line + context + 1).min(total_lines);

                                if match_idx > 0 {
                                    println!("...");
                                }

                                for i in context_start..context_end {
                                    let marker = if i == *match_line { ">" } else { " "};
                                    if show_numbers {
                                        println!("{} {:4}  {}", marker, i + 1, lines[i]);
                                    } else {
                                        println!("{}{}", marker, lines[i]);
                                    }
                                }
                            }
                        }
                    } else {
                        // Show line range
                        println!("\n{} Lines {}-{} of {}\n", "CONTENT".bold(), start + 1, end, total_lines);
                        
                        for (i, line) in lines.iter().enumerate().skip(start).take(end - start) {
                            if show_numbers {
                                println!("{:4}  {}", i + 1, line);
                            } else {
                                println!("{}", line);
                            }
                        }
                    }
                } else {
                    println!("\n  {} {}", "[Warning]".yellow(), format!("File not found on disk: {}", full_path.display()).dimmed());
                    println!("  {}", format!("Run 'codegraph pull {}' to re-clone the repository.", repo_id).dimmed());
                }
            }

            println!();
        }
        None => {
            println!("{}", format!("File not found: {} in {}", file_path, repo_id).red());
        }
    }

    Ok(())
}

/// Parse a line range string like "10-50" or "100-"
fn parse_range(range: &str, total_lines: usize) -> AnyResult<(usize, usize)> {
    let range = range.trim();
    
    if range.contains('-') {
        let parts: Vec<&str> = range.split('-').collect();
        let start = if parts[0].is_empty() {
            0
        } else {
            parts[0].parse::<usize>()?
                .saturating_sub(1) // Convert to 0-based
        };
        let end = if parts.len() > 1 && !parts[1].is_empty() {
            parts[1].parse::<usize>()?.min(total_lines)
        } else {
            total_lines
        };
        Ok((start, end))
    } else {
        // Single line number
        let line = range.parse::<usize>()?.saturating_sub(1);
        Ok((line, (line + 1).min(total_lines)))
    }
}

/// Search within multiple files in a repository
pub async fn grep(
    repo_id: &str, 
    pattern: &str, 
    glob: &str,
    show_numbers: bool,
    json: bool,
) -> AnyResult<()> {
    let db = Database::open()?;
    
    // Try to find repo by ID first, then by path (for local repos)
    let repo_path = if let Some(repo) = db.get_knowledge(repo_id)? {
        // Found by UUID
        match &repo.source {
            crate::db::KnowledgeSource::Local { path } => std::path::PathBuf::from(path),
            crate::db::KnowledgeSource::Github { .. } => crate::config::repos_dir()?.join(repo_id),
        }
    } else if let Some(repo) = db.get_knowledge_by_path(repo_id)? {
        // Found by path (local repos)
        match &repo.source {
            crate::db::KnowledgeSource::Local { path } => std::path::PathBuf::from(path),
            crate::db::KnowledgeSource::Github { .. } => crate::config::repos_dir()?.join(&repo.id),
        }
    } else {
        // Not in DB - try as a direct path
        let path = std::path::PathBuf::from(repo_id);
        if path.exists() {
            path
        } else {
            // Fall back to repos_dir (for github repos)
            crate::config::repos_dir()?.join(repo_id)
        }
    };

    if !repo_path.exists() {
        println!("{}", format!("Repository not found on disk: {}", repo_path.display()).red());
        println!("  {}", format!("Run 'codegraph pull {}' to re-clone the repository.", repo_id).dimmed());
        return Ok(());
    }

    let pattern_lower = pattern.to_lowercase();
    let mut matches: Vec<GrepMatch> = Vec::new();
    let mut files_searched = 0;
    let mut files_matched = 0;

    // Walk the repo directory and search files matching the glob
    for entry in walkdir(&repo_path, glob)? {
        files_searched += 1;
        
        let file_matches = search_file_in_repo(&repo_path, &entry, &pattern_lower)?;
        if !file_matches.is_empty() {
            files_matched += 1;
            matches.push(GrepMatch {
                path: entry,
                matches: file_matches,
            });
        }
    }

    if json {
        #[derive(serde::Serialize)]
        struct GrepResult {
            repo: String,
            pattern: String,
            files_searched: usize,
            files_matched: usize,
            total_matches: usize,
            results: Vec<GrepMatch>,
        }
        
        let total: usize = matches.iter().map(|m| m.matches.len()).sum();
        let result = GrepResult {
            repo: repo_id.to_string(),
            pattern: pattern.to_string(),
            files_searched,
            files_matched,
            total_matches: total,
            results: matches,
        };
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        if matches.is_empty() {
            println!("{}", format!("No matches found for '{}' in {}", pattern, repo_id).yellow());
            println!("  {} files searched", files_searched);
            return Ok(());
        }

        let total_matches: usize = matches.iter().map(|m| m.matches.len()).sum();
        println!("\n{} '{}' in {}\n", "GREP".bold(), pattern.yellow(), repo_id);
        println!("  {} files searched, {} files matched, {} total matches\n", 
            files_searched, files_matched, total_matches);

        for result in matches {
            println!("{}{}", "FILE".bold(), format!(": {}", result.path).dimmed());
            for m in result.matches {
                if show_numbers {
                    println!("  {:4}: {}", m.line, m.content);
                } else {
                    println!("  {}", m.content);
                }
            }
            println!();
        }
    }

    Ok(())
}

/// Walk directory and collect files matching glob pattern
fn walkdir(root: &std::path::Path, glob: &str) -> AnyResult<Vec<String>> {
    let mut files = Vec::new();
    let pattern = glob_pattern_to_regex(glob);
    
    fn walk(
        dir: &std::path::Path, 
        pattern: &regex::Regex, 
        files: &mut Vec<String>,
        root: &std::path::Path,
    ) -> std::io::Result<()> {
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    // Skip common non-source directories
                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    if !name.starts_with('.') 
                        && name != "node_modules" 
                        && name != "target" 
                        && name != "dist"
                        && name != "build"
                        && name != "__pycache__"
                    {
                        walk(&path, pattern, files, root)?;
                    }
                } else if path.is_file() {
                    let relative = path.strip_prefix(root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .replace('\\', "/");
                    if pattern.is_match(&relative) {
                        files.push(relative);
                    }
                }
            }
        }
        Ok(())
    }
    
    walk(root, &pattern, &mut files, root)?;
    files.sort();
    Ok(files)
}

    /// Convert glob pattern to regex
fn glob_pattern_to_regex(glob: &str) -> regex::Regex {
    let mut pattern = String::new();
    pattern.push('^');
    
    // Handle file extension patterns like *.rs, *.toml, etc.
    // and treat * as matching everything
    let glob = if glob == "*" || glob == "**" {
        "**".to_string()
    } else if !glob.contains('/') && (glob.starts_with('*') || glob.ends_with('*')) {
        // Simple pattern like *.rs -> **/*.rs (match in any dir)
        if glob.starts_with('*') {
            format!("**/{}", glob)
        } else {
            format!("**/{}", glob)
        }
    } else {
        glob.to_string()
    };
    
    let parts: Vec<&str> = glob.split('/').collect();
    for (i, part) in parts.iter().enumerate() {
        if i > 0 {
            pattern.push('/');
        }
        
        if *part == "**" {
            // ** matches any path including subdirectories
            pattern.push_str(".*");
        } else if *part == "*" {
            pattern.push_str("[^/]*");
        } else {
            // Escape special chars and replace * with match-anything
            for c in part.chars() {
                match c {
                    '*' => pattern.push_str(".*"),
                    '?' => pattern.push('.'),
                    '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '|' | '^' | '$' => {
                        pattern.push('\\');
                        pattern.push(c);
                    }
                    _ => pattern.push(c),
                }
            }
        }
    }
    
    pattern.push('$');
    regex::Regex::new(&pattern).unwrap_or_else(|_| regex::Regex::new("^.*$").unwrap())
}

/// Search a single file in a repository
fn search_file_in_repo(
    repo_path: &std::path::Path, 
    relative_path: &str, 
    pattern: &str,
) -> AnyResult<Vec<GrepMatchLine>> {
    let full_path = repo_path.join(relative_path);
    search_file_content(&full_path, pattern)
}

/// Search file content
fn search_file_content(path: &std::path::Path, pattern: &str) -> AnyResult<Vec<GrepMatchLine>> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Ok(Vec::new()), // Binary or unreadable file
    };
    
    let mut matches = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    
    for (i, line) in lines.iter().enumerate() {
        if line.to_lowercase().contains(pattern) {
            matches.push(GrepMatchLine {
                line: i + 1,
                content: line.to_string(),
            });
        }
    }
    
    Ok(matches)
}

#[derive(Debug, Clone, serde::Serialize)]
struct GrepMatch {
    path: String,
    matches: Vec<GrepMatchLine>,
}

#[derive(Debug, Clone, serde::Serialize)]
struct GrepMatchLine {
    line: usize,
    content: String,
}

/// Delete an indexed repository
pub async fn delete_repo(id: &str) -> AnyResult<()> {
    let db = Database::open()?;

    // Verify exists
    let repo = db.get_knowledge(id)?;
    if repo.is_none() {
        println!("{}", format!("Repository not found: {}", id).red());
        return Ok(());
    }

    // Delete
    db.delete_knowledge(id)?;

    println!("{}", format!("Deleted repository: {}", id).green());
    Ok(())
}

/// Show usage statistics
pub async fn show_stats() -> AnyResult<()> {
    let db = Database::open()?;
    let repos = db.list_knowledge()?;

    let total_files: i32 = repos.iter().map(|r| r.file_count).sum();
    let processed = repos.iter().filter(|r| r.state == crate::db::KnowledgeState::Processed).count();

    println!("\n{} {}\n", "STATISTICS".bold(), "\n");

    println!("  {:20} {}", "Total Repositories".dimmed(), repos.len());
    println!("  {:20} {}", "Processed".dimmed(), processed);
    println!("  {:20} {}", "Total Files".dimmed(), total_files);

    println!();
    Ok(())
}

/// Configuration management
pub async fn config(subcommand: ConfigCommands) -> AnyResult<()> {
    match subcommand {
        ConfigCommands::Get { key } => {
            let value = get_config_value(&key)?;
            println!("{} = {}", key, value);
        }
        ConfigCommands::Set { key, value } => {
            set_config_value(&key, &value)?;
            println!("{} {} = {}", "Set".green(), key, value);
        }
        ConfigCommands::Ls => {
            let config = load_config()?;
            println!("\n{} CONFIGURATION\n", "CURRENT".bold());
            println!("  {:30} {}", "llm_endpoint".dimmed(), config.llm_endpoint);
            println!("  {:30} {}", "llm_model".dimmed(), config.llm_model);
            println!("  {:30} {:?}", "llm_api_type".dimmed(), config.llm_api_type);
            println!("  {:30} {}", "llm_api_key".dimmed(), "[hidden]");
            println!("  {:30} {}", "concurrency".dimmed(), config.concurrency);
            println!("  {:30} {}", "max_file_tokens".dimmed(), config.max_file_tokens);
            println!("  {:30} {}", "log_level".dimmed(), config.log_level);
            println!();
        }
    }

    Ok(())
}
