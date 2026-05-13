//! CodeGraph Library
//!
//! Core functionality for the code knowledge graph engine.

pub mod cli;
pub mod config;
pub mod db;
pub mod error;
pub mod ingest;
pub mod llm;
pub mod queue;

pub use error::Error;
