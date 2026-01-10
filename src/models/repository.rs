use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The source forge where a repository is hosted
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    GitHub,
    GitLab,
    Codeberg,
}

/// A repository with all indexed metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    /// Unique identifier (source:owner/repo)
    pub id: String,

    /// Source forge
    pub source: Source,

    /// Full name (owner/repo)
    pub full_name: String,

    /// Web URL
    pub url: String,

    /// Repository description
    pub description: Option<String>,

    /// Topics/tags
    pub topics: Vec<String>,

    /// Primary language
    pub language: Option<String>,

    /// Language breakdown (language -> bytes)
    #[serde(default)]
    pub languages: HashMap<String, u64>,

    /// Star count
    pub stars: u32,

    /// Fork count
    pub forks: u32,

    /// Watcher count
    pub watchers: u32,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp (any activity)
    pub updated_at: DateTime<Utc>,

    /// Last push timestamp
    pub pushed_at: DateTime<Utc>,

    /// Open issue count
    pub open_issues: u32,

    /// Commits in last 30 days (derived)
    #[serde(default)]
    pub recent_commits_30d: u32,

    /// Last release date
    pub last_release: Option<DateTime<Utc>>,

    /// Computed activity score (0-100)
    #[serde(default)]
    pub activity_score: f32,

    /// When this record was indexed
    pub indexed_at: DateTime<Utc>,

    /// Whether the repository is archived
    #[serde(default)]
    pub archived: bool,

    /// Whether the repository is a fork
    #[serde(default)]
    pub is_fork: bool,

    /// License SPDX identifier
    pub license: Option<String>,

    /// Homepage URL
    pub homepage: Option<String>,
}

impl Repository {
    /// Create a new repository ID from source and full name
    pub fn make_id(source: &Source, full_name: &str) -> String {
        let source_str = match source {
            Source::GitHub => "github",
            Source::GitLab => "gitlab",
            Source::Codeberg => "codeberg",
        };
        format!("{}:{}", source_str, full_name)
    }

    /// Check if the repository is considered "active" based on activity score
    pub fn is_active(&self, threshold: f32) -> bool {
        self.activity_score >= threshold
    }
}
