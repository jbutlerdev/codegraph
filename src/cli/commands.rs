//! CLI commands using Clap

use clap::{Parser, Subcommand, ValueHint};
use std::path::PathBuf;

/// CodeGraph - Local code knowledge graph engine
#[derive(Parser, Debug)]
#[command(
    name = "codegraph",
    about = "Local code knowledge graph engine",
    long_about = None,
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Index a GitHub repository
    Index {
        /// GitHub repository URL
        #[arg(value_hint = ValueHint::Url)]
        url: String,

        /// Branch to index (default: main)
        #[arg(short, long)]
        branch: Option<String>,

        /// GitHub personal access token (for private repos)
        #[arg(short, long, hide = true)]
        token: Option<String>,
    },

    /// Index a local directory
    Ingest {
        /// Path to local directory
        #[arg(value_hint = ValueHint::DirPath)]
        path: PathBuf,
        /// Force re-analysis of all files (bypass SHA check)
        #[arg(short, long)]
        force: bool,
    },

    /// List indexed repositories
    Ls,

    /// Search indexed code (full-text search)
    Search {
        /// Search query
        query: String,

        /// Filter by repository ID
        #[arg(short, long)]
        repo: Option<String>,

        /// Maximum results (default: 20)
        #[arg(short, long, default_value_t = 20)]
        limit: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Lookup keywords, classes, or functions
    Lookup {
        /// Search term
        term: String,

        /// Filter by repository ID
        #[arg(short, long)]
        repo: Option<String>,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Show file metadata and/or content
    Cat {
        /// Repository ID
        #[arg(short, long)]
        repo: String,

        /// File path within repository
        #[arg(short, long)]
        file: String,

        /// Show file content (from local clone)
        #[arg(short, long)]
        content: bool,

        /// Show line numbers
        #[arg(short, long)]
        numbers: bool,

        /// Line range to show (e.g., "10-50")
        #[arg(short, long)]
        range: Option<String>,

        /// Search within file content
        #[arg(short, long)]
        search: Option<String>,
    },

    /// Search within files in a repository
    Grep {
        /// Repository ID
        #[arg(short, long)]
        repo: String,

        /// Search pattern
        pattern: String,

        /// File patterns (e.g., "*.rs" or "src/**/*.ts")
        #[arg(short, long, default_value = "*")]
        glob: String,

        /// Show line numbers
        #[arg(short, long)]
        numbers: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Delete an indexed repository
    Delete {
        /// Repository ID to delete
        id: String,
    },

    /// Re-index an existing repository
    Pull {
        /// Repository ID to re-index
        id: String,
    },

    /// Show usage statistics
    Stats,

    /// Find where an entity is defined
    Defines {
        /// Entity type (class, function, module)
        #[arg(value_enum, default_value = "class")]
        entity_type: EntityType,

        /// Entity name
        name: String,

        /// Repository ID (optional, searches all)
        #[arg(short, long)]
        repo: Option<String>,
    },

    /// Find files that reference/use an entity
    Uses {
        /// Entity type (class, function, module)
        #[arg(value_enum, default_value = "class")]
        entity_type: EntityType,

        /// Entity name
        name: String,

        /// Repository ID (optional, searches all)
        #[arg(short, long)]
        repo: Option<String>,
    },

    /// Get dependencies of a file
    Deps {
        /// Repository ID
        #[arg(short, long)]
        repo: String,

        /// File path
        #[arg(short, long)]
        file: String,
    },

    /// Get files that depend on a file
    Dependents {
        /// Repository ID
        #[arg(short, long)]
        repo: String,

        /// File path
        #[arg(short, long)]
        file: String,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        subcommand: ConfigCommands,
    },
}

/// Configuration subcommands
#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Get a config value
    Get {
        /// Config key
        key: String,
    },
    /// Set a config value
    Set {
        /// Config key
        key: String,
        /// Config value
        value: String,
    },
    /// List all config
    Ls,
}

/// Entity type for relationship queries
#[derive(clap::ValueEnum, Debug, Clone)]
pub enum EntityType {
    Class,
    Function,
    Module,
}

impl EntityType {
    pub fn as_str(&self) -> &'static str {
        match self {
            EntityType::Class => "class",
            EntityType::Function => "function",
            EntityType::Module => "module",
        }
    }
}
