//! GitHub GraphQL API client

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use reqwest::{header, Client};
use std::collections::HashMap;

use super::graphql::{
    CommitsResponse, GraphQLRequest, GraphQLResponse, ReleaseResponse, RepositoryNode,
    RepositoryResponse, SearchResponse, COMMITS_QUERY, RELEASE_QUERY, REPOSITORY_QUERY,
    SEARCH_QUERY,
};
use super::rate_limit::{with_retry, RateLimiter};
use super::RepositorySource;
use crate::models::{Repository, Source};

/// GitHub API client for fetching repository data via GraphQL
pub struct GitHubClient {
    client: Client,
    token: String,
    rate_limiter: RateLimiter,
}

impl GitHubClient {
    /// Create a new GitHub client with the given token
    pub fn new(token: String) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::AUTHORIZATION,
            header::HeaderValue::from_str(&format!("Bearer {}", token))
                .context("Invalid token format")?,
        );
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static("awesomeify/0.1"),
        );

        let client = Client::builder()
            .default_headers(headers)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            token,
            rate_limiter: RateLimiter::new(100), // Keep 100 requests in reserve
        })
    }

    /// Create from GITHUB_TOKEN environment variable
    pub fn from_env() -> Result<Self> {
        let token = std::env::var("GITHUB_TOKEN")
            .context("GITHUB_TOKEN environment variable not set")?;
        Self::new(token)
    }

    /// Check if the client has a valid token
    pub fn is_authenticated(&self) -> bool {
        !self.token.is_empty()
    }

    /// Get remaining rate limit
    pub async fn rate_limit_remaining(&self) -> i64 {
        self.rate_limiter.remaining().await
    }

    /// Execute a GraphQL query
    async fn execute<T>(&self, query: &'static str, variables: serde_json::Value) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        let request = GraphQLRequest { query, variables };

        let response = self
            .client
            .post("https://api.github.com/graphql")
            .json(&request)
            .send()
            .await
            .context("Failed to send GraphQL request")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "GitHub API error: {} - {}",
                status,
                body
            ));
        }

        let graphql_response: GraphQLResponse<T> = response
            .json()
            .await
            .context("Failed to parse GraphQL response")?;

        if let Some(errors) = graphql_response.errors {
            let error_messages: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
            return Err(anyhow!("GraphQL errors: {}", error_messages.join(", ")));
        }

        graphql_response
            .data
            .ok_or_else(|| anyhow!("No data in GraphQL response"))
    }

    /// Convert a GraphQL repository node to our Repository model
    fn node_to_repository(&self, node: RepositoryNode) -> Repository {
        let languages: HashMap<String, u64> = node
            .languages
            .edges
            .into_iter()
            .map(|edge| (edge.node.name, edge.size as u64))
            .collect();

        let topics: Vec<String> = node
            .repository_topics
            .nodes
            .into_iter()
            .map(|t| t.topic.name)
            .collect();

        let created_at = DateTime::parse_from_rfc3339(&node.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let updated_at = DateTime::parse_from_rfc3339(&node.updated_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());

        let pushed_at = node
            .pushed_at
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or(updated_at);

        let last_release = node
            .latest_release
            .and_then(|r| r.published_at)
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Repository {
            id: Repository::make_id(&Source::GitHub, &node.name_with_owner),
            source: Source::GitHub,
            full_name: node.name_with_owner,
            url: node.url,
            description: node.description,
            topics,
            language: node.primary_language.map(|l| l.name),
            languages,
            stars: node.stargazer_count as u32,
            forks: node.fork_count as u32,
            watchers: node.watchers.total_count as u32,
            created_at,
            updated_at,
            pushed_at,
            open_issues: node.issues.total_count as u32,
            recent_commits_30d: 0, // Will be fetched separately if needed
            last_release,
            activity_score: 0.0, // Will be calculated later
            indexed_at: Utc::now(),
            archived: node.is_archived,
            is_fork: node.is_fork,
            license: node.license_info.and_then(|l| l.spdx_id),
            homepage: node.homepage_url,
        }
    }

    /// Search repositories with pagination
    pub async fn search_all(
        &self,
        query: &str,
        limit: usize,
        progress_callback: Option<Box<dyn Fn(usize, usize) + Send + Sync>>,
    ) -> Result<Vec<Repository>> {
        let mut repositories = Vec::new();
        let mut cursor: Option<String> = None;
        let page_size = 100.min(limit); // GitHub max is 100 per page

        loop {
            if repositories.len() >= limit {
                break;
            }

            let remaining = limit - repositories.len();
            let fetch_count = remaining.min(page_size);

            let variables = serde_json::json!({
                "query": query,
                "first": fetch_count,
                "after": cursor,
            });

            let response: SearchResponse = with_retry(&self.rate_limiter, 3, || {
                self.execute(SEARCH_QUERY, variables.clone())
            })
            .await?;

            // Update rate limit
            if let Some(rate_limit) = &response.rate_limit {
                self.rate_limiter.update(rate_limit).await;
            }

            let search = response.search;

            // Convert nodes to repositories
            for node_opt in search.nodes {
                if let Some(node) = node_opt {
                    let repo = self.node_to_repository(node);
                    repositories.push(repo);

                    if repositories.len() >= limit {
                        break;
                    }
                }
            }

            // Report progress
            if let Some(ref callback) = progress_callback {
                callback(repositories.len(), search.repository_count as usize);
            }

            tracing::info!(
                fetched = repositories.len(),
                total = search.repository_count,
                "Search progress"
            );

            // Check for more pages
            if !search.page_info.has_next_page || repositories.len() >= limit {
                break;
            }

            cursor = search.page_info.end_cursor;
        }

        Ok(repositories)
    }
}

#[async_trait]
impl RepositorySource for GitHubClient {
    /// Fetch a single repository by owner/name
    async fn get_repository(&self, owner: &str, name: &str) -> Result<Repository> {
        let variables = serde_json::json!({
            "owner": owner,
            "name": name,
        });

        let response: RepositoryResponse = with_retry(&self.rate_limiter, 3, || {
            self.execute(REPOSITORY_QUERY, variables.clone())
        })
        .await?;

        // Update rate limit
        if let Some(rate_limit) = &response.rate_limit {
            self.rate_limiter.update(rate_limit).await;
        }

        let node = response
            .repository
            .ok_or_else(|| anyhow!("Repository not found: {}/{}", owner, name))?;

        Ok(self.node_to_repository(node))
    }

    /// Search repositories with a query string
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<Repository>> {
        self.search_all(query, limit, None).await
    }

    /// Get recent commit count for a repository
    async fn get_recent_commits(&self, owner: &str, name: &str, days: u32) -> Result<u32> {
        let since = Utc::now() - Duration::days(days as i64);

        let variables = serde_json::json!({
            "owner": owner,
            "name": name,
            "since": since.to_rfc3339(),
        });

        let response: CommitsResponse = with_retry(&self.rate_limiter, 3, || {
            self.execute(COMMITS_QUERY, variables.clone())
        })
        .await?;

        // Update rate limit
        if let Some(rate_limit) = &response.rate_limit {
            self.rate_limiter.update(rate_limit).await;
        }

        let count = response
            .repository
            .and_then(|r| r.default_branch_ref)
            .and_then(|b| b.target)
            .and_then(|t| t.history)
            .map(|h| h.total_count as u32)
            .unwrap_or(0);

        Ok(count)
    }

    /// Get the latest release date for a repository
    async fn get_latest_release(&self, owner: &str, name: &str) -> Result<Option<DateTime<Utc>>> {
        let variables = serde_json::json!({
            "owner": owner,
            "name": name,
        });

        let response: ReleaseResponse = with_retry(&self.rate_limiter, 3, || {
            self.execute(RELEASE_QUERY, variables.clone())
        })
        .await?;

        // Update rate limit
        if let Some(rate_limit) = &response.rate_limit {
            self.rate_limiter.update(rate_limit).await;
        }

        let release_date = response
            .repository
            .and_then(|r| r.latest_release)
            .and_then(|r| r.published_at)
            .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        Ok(release_date)
    }
}

/// Parse a GitHub repository URL into owner and name
pub fn parse_github_url(url: &str) -> Option<(String, String)> {
    // Handle various formats:
    // https://github.com/owner/repo
    // https://github.com/owner/repo.git
    // git@github.com:owner/repo.git
    // github.com/owner/repo

    let url = url
        .trim()
        .trim_end_matches('/')
        .trim_end_matches(".git");

    // Try HTTPS format
    if let Some(path) = url.strip_prefix("https://github.com/") {
        let parts: Vec<&str> = path.splitn(3, '/').collect();
        if parts.len() >= 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }

    // Try SSH format
    if let Some(path) = url.strip_prefix("git@github.com:") {
        let parts: Vec<&str> = path.splitn(3, '/').collect();
        if parts.len() >= 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }

    // Try plain format
    if let Some(path) = url.strip_prefix("github.com/") {
        let parts: Vec<&str> = path.splitn(3, '/').collect();
        if parts.len() >= 2 {
            return Some((parts[0].to_string(), parts[1].to_string()));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_url_https() {
        let result = parse_github_url("https://github.com/rust-lang/rust");
        assert_eq!(result, Some(("rust-lang".to_string(), "rust".to_string())));
    }

    #[test]
    fn test_parse_github_url_https_with_git() {
        let result = parse_github_url("https://github.com/rust-lang/rust.git");
        assert_eq!(result, Some(("rust-lang".to_string(), "rust".to_string())));
    }

    #[test]
    fn test_parse_github_url_ssh() {
        let result = parse_github_url("git@github.com:rust-lang/rust.git");
        assert_eq!(result, Some(("rust-lang".to_string(), "rust".to_string())));
    }

    #[test]
    fn test_parse_github_url_plain() {
        let result = parse_github_url("github.com/rust-lang/rust");
        assert_eq!(result, Some(("rust-lang".to_string(), "rust".to_string())));
    }

    #[test]
    fn test_parse_github_url_with_trailing_slash() {
        let result = parse_github_url("https://github.com/rust-lang/rust/");
        assert_eq!(result, Some(("rust-lang".to_string(), "rust".to_string())));
    }

    #[test]
    fn test_parse_github_url_invalid() {
        let result = parse_github_url("https://gitlab.com/user/repo");
        assert_eq!(result, None);
    }
}
