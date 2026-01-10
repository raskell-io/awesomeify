use async_trait::async_trait;
use crate::models::Repository;

/// Trait for repository sources (GitHub, GitLab, etc.)
#[async_trait]
pub trait RepositorySource {
    /// Fetch a single repository by owner/name
    async fn get_repository(&self, owner: &str, name: &str) -> anyhow::Result<Repository>;

    /// Search repositories with a query string
    async fn search(&self, query: &str, limit: usize) -> anyhow::Result<Vec<Repository>>;

    /// Get recent commit count for a repository
    async fn get_recent_commits(&self, owner: &str, name: &str, days: u32) -> anyhow::Result<u32>;

    /// Get the latest release date for a repository
    async fn get_latest_release(&self, owner: &str, name: &str) -> anyhow::Result<Option<chrono::DateTime<chrono::Utc>>>;
}
