//! GraphQL queries and response types for GitHub API

use serde::{Deserialize, Serialize};

/// GraphQL query for fetching a single repository
pub const REPOSITORY_QUERY: &str = r#"
query GetRepository($owner: String!, $name: String!) {
  repository(owner: $owner, name: $name) {
    ...RepoFields
  }
  rateLimit {
    remaining
    resetAt
  }
}

fragment RepoFields on Repository {
  nameWithOwner
  url
  description
  homepageUrl
  stargazerCount
  forkCount
  watchers {
    totalCount
  }
  issues(states: OPEN) {
    totalCount
  }
  createdAt
  updatedAt
  pushedAt
  isArchived
  isFork
  primaryLanguage {
    name
  }
  languages(first: 10, orderBy: {field: SIZE, direction: DESC}) {
    edges {
      size
      node {
        name
      }
    }
  }
  licenseInfo {
    spdxId
  }
  repositoryTopics(first: 20) {
    nodes {
      topic {
        name
      }
    }
  }
  latestRelease {
    publishedAt
  }
  defaultBranchRef {
    target {
      ... on Commit {
        history(first: 1) {
          totalCount
        }
      }
    }
  }
}
"#;

/// GraphQL query for searching repositories
pub const SEARCH_QUERY: &str = r#"
query SearchRepositories($query: String!, $first: Int!, $after: String) {
  search(query: $query, type: REPOSITORY, first: $first, after: $after) {
    repositoryCount
    pageInfo {
      endCursor
      hasNextPage
    }
    nodes {
      ... on Repository {
        ...RepoFields
      }
    }
  }
  rateLimit {
    remaining
    resetAt
  }
}

fragment RepoFields on Repository {
  nameWithOwner
  url
  description
  homepageUrl
  stargazerCount
  forkCount
  watchers {
    totalCount
  }
  issues(states: OPEN) {
    totalCount
  }
  createdAt
  updatedAt
  pushedAt
  isArchived
  isFork
  primaryLanguage {
    name
  }
  languages(first: 10, orderBy: {field: SIZE, direction: DESC}) {
    edges {
      size
      node {
        name
      }
    }
  }
  licenseInfo {
    spdxId
  }
  repositoryTopics(first: 20) {
    nodes {
      topic {
        name
      }
    }
  }
  latestRelease {
    publishedAt
  }
  defaultBranchRef {
    target {
      ... on Commit {
        history(first: 1) {
          totalCount
        }
      }
    }
  }
}
"#;

/// GraphQL query for fetching recent commits
pub const COMMITS_QUERY: &str = r#"
query GetRecentCommits($owner: String!, $name: String!, $since: GitTimestamp!) {
  repository(owner: $owner, name: $name) {
    defaultBranchRef {
      target {
        ... on Commit {
          history(since: $since) {
            totalCount
          }
        }
      }
    }
  }
  rateLimit {
    remaining
    resetAt
  }
}
"#;

/// GraphQL query for fetching latest release
pub const RELEASE_QUERY: &str = r#"
query GetLatestRelease($owner: String!, $name: String!) {
  repository(owner: $owner, name: $name) {
    latestRelease {
      publishedAt
    }
  }
  rateLimit {
    remaining
    resetAt
  }
}
"#;

// Response types

#[derive(Debug, Deserialize)]
pub struct GraphQLResponse<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLError {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryResponse {
    pub repository: Option<RepositoryNode>,
    pub rate_limit: Option<RateLimit>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub search: SearchResult,
    pub rate_limit: Option<RateLimit>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub repository_count: i64,
    pub page_info: PageInfo,
    pub nodes: Vec<Option<RepositoryNode>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub end_cursor: Option<String>,
    pub has_next_page: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryNode {
    pub name_with_owner: String,
    pub url: String,
    pub description: Option<String>,
    pub homepage_url: Option<String>,
    pub stargazer_count: i64,
    pub fork_count: i64,
    pub watchers: CountNode,
    pub issues: CountNode,
    pub created_at: String,
    pub updated_at: String,
    pub pushed_at: Option<String>,
    pub is_archived: bool,
    pub is_fork: bool,
    pub primary_language: Option<LanguageNode>,
    pub languages: LanguagesConnection,
    pub license_info: Option<LicenseNode>,
    pub repository_topics: TopicsConnection,
    pub latest_release: Option<ReleaseNode>,
    pub default_branch_ref: Option<BranchRef>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CountNode {
    pub total_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct LanguageNode {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct LanguagesConnection {
    pub edges: Vec<LanguageEdge>,
}

#[derive(Debug, Deserialize)]
pub struct LanguageEdge {
    pub size: i64,
    pub node: LanguageNode,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LicenseNode {
    pub spdx_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TopicsConnection {
    pub nodes: Vec<TopicNode>,
}

#[derive(Debug, Deserialize)]
pub struct TopicNode {
    pub topic: TopicName,
}

#[derive(Debug, Deserialize)]
pub struct TopicName {
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseNode {
    pub published_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BranchRef {
    pub target: Option<CommitTarget>,
}

#[derive(Debug, Deserialize)]
pub struct CommitTarget {
    pub history: Option<CountNode>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimit {
    pub remaining: i64,
    pub reset_at: String,
}

// Commits response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitsResponse {
    pub repository: Option<CommitsRepository>,
    pub rate_limit: Option<RateLimit>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitsRepository {
    pub default_branch_ref: Option<BranchRef>,
}

// Release response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseResponse {
    pub repository: Option<ReleaseRepository>,
    pub rate_limit: Option<RateLimit>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseRepository {
    pub latest_release: Option<ReleaseNode>,
}

/// GraphQL request body
#[derive(Debug, Serialize)]
pub struct GraphQLRequest {
    pub query: &'static str,
    pub variables: serde_json::Value,
}
