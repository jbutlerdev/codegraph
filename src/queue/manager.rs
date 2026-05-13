//! Queue manager - manages job queue state using sled

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sled::{Db, Tree};
use std::path::Path;

use crate::config::queue_dir;

/// Job state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "state")]
pub enum JobState {
    Pending,
    Processing { started_at: String },
    Completed { finished_at: String },
    Failed { error: String },
}

/// Job record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub kind: String,
    pub payload: String,
    pub state: JobState,
    pub priority: i32,
    pub created_at: String,
    pub updated_at: String,
}

/// Queue manager
pub struct QueueManager {
    pub(crate) db: Db,
    pub(crate) pending: Tree,
    pub(crate) processing: Tree,
    pub(crate) completed: Tree,
}

impl QueueManager {
    /// Open the queue at the default location
    pub fn open() -> Result<Self> {
        let dir = queue_dir()?;
        Self::open_at(&dir)
    }

    /// Open the queue at a specific path
    pub fn open_at(path: &Path) -> Result<Self> {
        let db = sled::open(path)?;

        let pending = db.open_tree("pending")?;
        let processing = db.open_tree("processing")?;
        let completed = db.open_tree("completed")?;

        Ok(Self {
            db,
            pending,
            processing,
            completed,
        })
    }

    /// Enqueue a job
    pub fn enqueue(&self, job: &Job) -> Result<()> {
        let key = job.id.as_bytes();
        let value = serde_json::to_vec(job)?;

        self.pending.insert(key, value)?;
        self.pending.flush()?;

        Ok(())
    }

    /// Dequeue the next pending job
    pub fn dequeue(&self) -> Result<Option<Job>> {
        // Get first item from pending (ordered by priority)
        let item = self.pending.iter()
            .next()
            .transpose()?;

        if let Some((key, value)) = item {
            let mut job: Job = serde_json::from_slice(&value)?;

            // Move to processing
            self.pending.remove(&key)?;
            job.state = JobState::Processing {
                started_at: chrono::Utc::now().to_rfc3339(),
            };
            job.updated_at = chrono::Utc::now().to_rfc3339();

            let new_value = serde_json::to_vec(&job)?;
            self.processing.insert(&key, new_value)?;
            self.processing.flush()?;

            Ok(Some(job))
        } else {
            Ok(None)
        }
    }

    /// Complete a job
    pub fn complete(&self, job_id: &str) -> Result<()> {
        let key = job_id.as_bytes();

        if let Some(value) = self.processing.remove(&key)? {
            let mut job: Job = serde_json::from_slice(&value)?;
            job.state = JobState::Completed {
                finished_at: chrono::Utc::now().to_rfc3339(),
            };
            job.updated_at = chrono::Utc::now().to_rfc3339();

            let new_value = serde_json::to_vec(&job)?;
            self.completed.insert(&key, new_value)?;
            self.completed.flush()?;
        }

        Ok(())
    }

    /// Fail a job
    pub fn fail(&self, job_id: &str, error: &str) -> Result<()> {
        let key = job_id.as_bytes();

        if let Some(value) = self.processing.remove(&key)? {
            let mut job: Job = serde_json::from_slice(&value)?;
            job.state = JobState::Failed {
                error: error.to_string(),
            };
            job.updated_at = chrono::Utc::now().to_rfc3339();

            let new_value = serde_json::to_vec(&job)?;
            self.processing.insert(&key, new_value)?;
            self.processing.flush()?;
        }

        Ok(())
    }

    /// Get job status
    pub fn get_job(&self, job_id: &str) -> Result<Option<Job>> {
        let key = job_id.as_bytes();

        // Check pending
        if let Some(value) = self.pending.get(&key)? {
            return Ok(Some(serde_json::from_slice(&value)?));
        }

        // Check processing
        if let Some(value) = self.processing.get(&key)? {
            return Ok(Some(serde_json::from_slice(&value)?));
        }

        // Check completed
        if let Some(value) = self.completed.get(&key)? {
            return Ok(Some(serde_json::from_slice(&value)?));
        }

        Ok(None)
    }

    /// Get count of pending jobs
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Get count of processing jobs
    pub fn processing_count(&self) -> usize {
        self.processing.len()
    }
}
