//! CodeGraph - Local code knowledge graph engine
//!
//! A Rust reimplementation of Bytebell's core functionality:
//! - Embedded SQLite + FTS5 storage (replaces Neo4j + MongoDB)
//! - In-process job queue (sled-backed)
//! - CLI-first search and retrieval

mod cli;
mod config;
mod db;
mod error;
mod ingest;
mod llm;
mod queue;

use anyhow::Result;
use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .init();

    info!("CodeGraph starting...");

    // Parse CLI arguments
    let args = cli::Cli::parse();

    // Execute command
    match args.command {
        cli::Commands::Index { url, branch, token } => {
            ingest::github::index_repo(&url, branch.as_deref(), token.as_deref()).await?;
        }
        cli::Commands::Ingest { path } => {
            ingest::local::ingest_path(&path).await?;
        }
        cli::Commands::Ls => {
            cli::list_repos().await?;
        }
        cli::Commands::Search { query, repo, limit, json } => {
            cli::search(&query, repo.as_deref(), limit, json).await?;
        }
        cli::Commands::Lookup { term, repo, json } => {
            cli::lookup(&term, repo.as_deref(), json).await?;
        }
        cli::Commands::Cat { repo, file, content, numbers, range, search } => {
            cli::cat_file(&repo, &file, content, numbers, range.as_deref(), search.as_deref()).await?;
        }
        cli::Commands::Grep { repo, pattern, glob, numbers, json } => {
            cli::grep(&repo, &pattern, &glob, numbers, json).await?;
        }
        cli::Commands::Delete { id } => {
            cli::delete_repo(&id).await?;
        }
        cli::Commands::Pull { id } => {
            ingest::github::pull_repo(&id).await?;
        }
        cli::Commands::Stats => {
            cli::show_stats().await?;
        }
        cli::Commands::Defines { entity_type, name, repo } => {
            cli::find_definition(entity_type.as_str(), &name, repo.as_deref()).await?;
        }
        cli::Commands::Uses { entity_type, name, repo } => {
            cli::find_usages(entity_type.as_str(), &name, repo.as_deref()).await?;
        }
        cli::Commands::Deps { repo, file } => {
            cli::get_dependencies(&repo, &file).await?;
        }
        cli::Commands::Dependents { repo, file } => {
            cli::get_dependents(&repo, &file).await?;
        }
        cli::Commands::Config { subcommand } => {
            cli::config(subcommand).await?;
        }
    }

    Ok(())
}
