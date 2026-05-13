//! Entity relationship commands

use crate::db::Database;
use anyhow::Result as AnyResult;
use colored::Colorize;

/// Find where an entity is defined
pub async fn find_definition(entity_type: &str, name: &str, repo_id: Option<&str>, show_all: bool) -> AnyResult<()> {
    let db = Database::open()?;

    // Find the entity first
    let _entity_id = match entity_type {
        "class" => {
            db.get_or_create_class(name).ok()
        }
        "function" => {
            db.get_or_create_function(name).ok()
        }
        "module" => {
            db.get_or_create_module(name, false).ok()
        }
        _ => {
            println!("{}", format!("Unknown entity type: {}", entity_type).red());
            return Ok(());
        }
    };

    // Find where it's defined
    let (defines, usages) = db.get_entity_complete_references(entity_type, name, repo_id)?;

    if show_all {
        // Combined view: show both definition and all usages
        println!("\n{} '{}' (type: {})\n", "COMPLETE VIEW".bold(), name.bold(), entity_type);
        
        if let Some(def) = defines {
            println!("  {} {}", "Defined in:".cyan().bold(), def.file_path);
        } else {
            println!("  {} {}", "Defined in:".cyan().bold(), "Not found".yellow());
        }
        
        println!("\n  {} {} file(s) use this entity\n", "USAGES".bold(), usages.len());
        
        if usages.is_empty() {
            println!("    {}", "No usages found".dimmed());
        } else {
            for usage in usages.iter().take(30) {
                println!("  {} {}", "->".dimmed(), usage.file_path);
            }
            if usages.len() > 30 {
                println!("  {} ... and {} more", "->".dimmed(), usages.len() - 30);
            }
        }
    } else {
        // Original view: just show definition
        println!("\n{} '{}' (type: {})\n", "DEFINITION".bold(), name.bold(), entity_type);

        if let Some(def) = defines {
            println!("  {} {}", "Defined in:".cyan(), def.file_path);
        } else {
            println!("  {}", "Not found in any file".yellow());
        }

        println!("\n{} {} file(s)\n", "REFERENCES".bold(), format!("({} usages)", usages.len()).dimmed());

        for usage in usages.iter().take(20) {
            println!("  {} {}", "->".dimmed(), usage.file_path);
        }

        if usages.len() > 20 {
            println!("  {} ... and {} more", "->".dimmed(), usages.len() - 20);
        }
    }

    println!();
    Ok(())
}

/// Find files that use/reference an entity
pub async fn find_usages(entity_type: &str, name: &str, repo_id: Option<&str>) -> AnyResult<()> {
    let db = Database::open()?;

    let usages = db.get_entity_usages(entity_type, name, repo_id)?;

    println!("\n{} '{}' (type: {})\n", "USAGES".bold(), name.bold(), entity_type);

    if usages.is_empty() {
        println!("  {}", "No usages found".yellow());
    } else {
        println!("  {} file(s) reference this entity\n", usages.len());
        for usage in usages.iter().take(30) {
            println!("  {} {}", "->".dimmed(), usage.file_path);
        }
        if usages.len() > 30 {
            println!("  {} ... and {} more", "->".dimmed(), usages.len() - 30);
        }
    }

    println!();
    Ok(())
}

/// Get dependencies of a file
pub async fn get_dependencies(repo_id: &str, file_path: &str) -> AnyResult<()> {
    let db = Database::open()?;

    // Find the file
    let file = db.get_file(repo_id, file_path)?;
    let file = match file {
        Some(f) => f,
        None => {
            println!("{}", format!("File not found: {} in {}", file_path, repo_id).red());
            return Ok(());
        }
    };

    let deps = db.get_file_dependencies(file.id)?;

    println!("\n{} {}\n", "DEPENDENCIES".bold(), file_path);

    if !deps.classes_defined.is_empty() {
        println!("  {} Classes defined:", "Defines".cyan());
        for class in &deps.classes_defined {
            println!("    - {}", class.split(':').next().unwrap_or(class));
        }
    }

    if !deps.functions_defined.is_empty() {
        println!("  {} Functions defined:", "Defines".cyan());
        for func in &deps.functions_defined {
            println!("    - {}", func.split(':').next().unwrap_or(func));
        }
    }

    if !deps.modules_defined.is_empty() {
        println!("  {} Modules defined:", "Defines".cyan());
        for module in &deps.modules_defined {
            println!("    - {}", module);
        }
    }

    println!();
    Ok(())
}

/// Get files that depend on a file
pub async fn get_dependents(repo_id: &str, file_path: &str) -> AnyResult<()> {
    let db = Database::open()?;

    // Find the file
    let file = db.get_file(repo_id, file_path)?;
    let file = match file {
        Some(f) => f,
        None => {
            println!("{}", format!("File not found: {} in {}", file_path, repo_id).red());
            return Ok(());
        }
    };

    let dependents = db.get_file_dependents(file.id, Some(repo_id), 50)?;

    println!("\n{} {}\n", "DEPENDENTS".bold(), file_path);

    if dependents.is_empty() {
        println!("  {}", "No files depend on this file".yellow());
    } else {
        println!("  {} file(s) depend on this file\n", dependents.len());
        for (path, _) in dependents {
            println!("  {} {}", "<-".dimmed(), path);
        }
    }

    println!();
    Ok(())
}

/// Find most depended-upon entities (top entities by usage count)
pub async fn find_top_entities(entity_type: &str, repo_id: Option<&str>, limit: usize) -> AnyResult<()> {
    let db = Database::open()?;

    let stats = db.get_top_entities_by_usage(entity_type, repo_id, limit as i32)?;

    let entity_label = match entity_type {
        "class" => "classes",
        "function" => "functions", 
        "module" => "modules",
        _ => entity_type,
    };

    println!("\n{} (type: {})\n", "MOST DEPENDED-UPON ENTITIES".bold(), entity_type);

    if stats.is_empty() {
        println!("  {}", "No entities found".yellow());
    } else {
        println!("  {} {} {}\n", 
            "Showing top".dimmed(), 
            stats.len(), 
            entity_label.dimmed()
        );
        
        for (i, stat) in stats.iter().enumerate() {
            // Extract clean name (without line numbers/signature details)
            let clean_name = stat.name.split('(').next().unwrap_or(&stat.name).trim().to_string();
            
            println!("  {}  {}  {}",
                format!("{}", i + 1).dimmed(),
                clean_name.bold(),
                format!("({} refs)", stat.usage_count).cyan()
            );
        }
    }

    println!();
    Ok(())
}
