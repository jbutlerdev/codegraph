//! Queue worker - processes jobs asynchronously

use anyhow::Result;
use std::future::Future;
use std::pin::Pin;
use tracing::{info, error};

use super::{Job, QueueManager};

/// Boxed future type for job handling
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = Result<T>> + Send>>;

/// Job handler trait
pub trait JobHandler: Send + Sync {
    fn job_kind(&self) -> &str;
    fn handle(&self, job: &Job) -> BoxFuture<()>;
}

/// Worker pool
pub struct WorkerPool {
    manager: QueueManager,
}

impl WorkerPool {
    /// Create a new worker pool
    pub fn new(manager: QueueManager) -> Self {
        Self { manager }
    }

    /// Start the worker pool
    pub async fn start(&self) {
        let manager = self.manager.clone();

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                if let Err(e) = Self::process_next(&manager).await {
                    error!("Error processing job: {}", e);
                }
            }
        });
    }

    /// Process the next pending job
    async fn process_next(manager: &QueueManager) -> Result<()> {
        if let Some(job) = manager.dequeue()? {
            info!("Processing job: {} ({})", job.id, job.kind);
            // For now, just mark as completed
            manager.complete(&job.id)?;
            info!("Completed job: {}", job.id);
        }

        Ok(())
    }
}

impl Clone for QueueManager {
    fn clone(&self) -> Self {
        // Re-open the queue - simplified for now
        QueueManager::open().expect("Failed to reopen queue")
    }
}
