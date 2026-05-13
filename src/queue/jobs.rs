//! Job types and payloads

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::Utc;

/// Job priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobPriority {
    Low = 10,
    Normal = 5,
    High = 1,
}

/// Job type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobType {
    GithubIndex,
    GithubPull,
    LocalIngest,
}

impl std::fmt::Display for JobType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobType::GithubIndex => write!(f, "github-index"),
            JobType::GithubPull => write!(f, "github-pull"),
            JobType::LocalIngest => write!(f, "local-ingest"),
        }
    }
}

/// GitHub index payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubIndexPayload {
    pub knowledge_id: String,
    pub repo_url: String,
    pub branch: Option<String>,
    pub git_token: Option<String>,
}

/// GitHub pull payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GithubPullPayload {
    pub knowledge_id: String,
}

/// Local ingest payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalIngestPayload {
    pub knowledge_id: String,
    pub source_path: String,
}

/// Job payload enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum JobPayload {
    #[serde(rename = "github-index")]
    GithubIndex(GithubIndexPayload),
    #[serde(rename = "github-pull")]
    GithubPull(GithubPullPayload),
    #[serde(rename = "local-ingest")]
    LocalIngest(LocalIngestPayload),
}

/// Create a new GitHub index job
pub fn new_github_index_job(
    knowledge_id: &str,
    repo_url: &str,
    branch: Option<&str>,
    git_token: Option<&str>,
    priority: JobPriority,
) -> super::Job {
    let payload = GithubIndexPayload {
        knowledge_id: knowledge_id.to_string(),
        repo_url: repo_url.to_string(),
        branch: branch.map(|s| s.to_string()),
        git_token: git_token.map(|s| s.to_string()),
    };

    super::Job {
        id: Uuid::new_v4().to_string(),
        kind: JobType::GithubIndex.to_string(),
        payload: serde_json::to_string(&JobPayload::GithubIndex(payload)).unwrap(),
        state: super::JobState::Pending,
        priority: priority as i32,
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
    }
}

/// Create a new GitHub pull job
pub fn new_github_pull_job(knowledge_id: &str, priority: JobPriority) -> super::Job {
    let payload = GithubPullPayload {
        knowledge_id: knowledge_id.to_string(),
    };

    super::Job {
        id: Uuid::new_v4().to_string(),
        kind: JobType::GithubPull.to_string(),
        payload: serde_json::to_string(&JobPayload::GithubPull(payload)).unwrap(),
        state: super::JobState::Pending,
        priority: priority as i32,
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
    }
}

/// Create a new local ingest job
pub fn new_local_ingest_job(knowledge_id: &str, source_path: &str, priority: JobPriority) -> super::Job {
    let payload = LocalIngestPayload {
        knowledge_id: knowledge_id.to_string(),
        source_path: source_path.to_string(),
    };

    super::Job {
        id: Uuid::new_v4().to_string(),
        kind: JobType::LocalIngest.to_string(),
        payload: serde_json::to_string(&JobPayload::LocalIngest(payload)).unwrap(),
        state: super::JobState::Pending,
        priority: priority as i32,
        created_at: Utc::now().to_rfc3339(),
        updated_at: Utc::now().to_rfc3339(),
    }
}
