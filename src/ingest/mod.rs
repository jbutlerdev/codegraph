//! Ingest module - repository ingestion

mod scanner;
mod analyzer;
mod big_file;
pub mod github;
pub mod local;

pub use scanner::*;
pub use analyzer::*;
