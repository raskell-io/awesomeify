//! Seed list parser for extracting repository URLs from awesome-lists and config files

use anyhow::{Context, Result};
use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use serde::Deserialize;
use std::collections::HashSet;
use std::path::Path;

use super::github::parse_github_url;

/// Seed configuration loaded from TOML
#[derive(Debug, Deserialize)]
pub struct SeedConfig {
    #[serde(default)]
    pub seeds: Vec<SeedEntry>,
    #[serde(default)]
    pub queries: Vec<QueryEntry>,
}

/// A seed source (awesome-list URL or local file)
#[derive(Debug, Deserialize)]
pub struct SeedEntry {
    pub name: String,
    #[serde(rename = "type")]
    pub seed_type: String,
    pub url: String,
}

/// A search query preset
#[derive(Debug, Deserialize)]
pub struct QueryEntry {
    pub name: String,
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    100
}

impl SeedConfig {
    /// Load seed configuration from a TOML file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read seed config: {}", path.display()))?;
        toml::from_str(&content)
            .with_context(|| format!("Failed to parse seed config: {}", path.display()))
    }
}

/// Extract GitHub repository URLs from markdown content
pub fn extract_github_urls(markdown: &str) -> Vec<(String, String)> {
    let mut urls = HashSet::new();
    let parser = Parser::new(markdown);

    let mut in_link = false;
    let mut current_url = String::new();

    for event in parser {
        match event {
            Event::Start(Tag::Link { dest_url, .. }) => {
                in_link = true;
                current_url = dest_url.to_string();
            }
            Event::End(TagEnd::Link) => {
                if in_link {
                    if let Some((owner, name)) = parse_github_url(&current_url) {
                        urls.insert((owner, name));
                    }
                }
                in_link = false;
                current_url.clear();
            }
            _ => {}
        }
    }

    urls.into_iter().collect()
}

/// Fetch and parse an awesome-list from a URL
pub async fn fetch_awesome_list(url: &str) -> Result<Vec<(String, String)>> {
    let client = reqwest::Client::builder()
        .user_agent("awesomeify/0.1")
        .build()
        .context("Failed to create HTTP client")?;

    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("Failed to fetch awesome-list: {}", url))?;

    if !response.status().is_success() {
        anyhow::bail!(
            "Failed to fetch awesome-list: {} - {}",
            url,
            response.status()
        );
    }

    let markdown = response
        .text()
        .await
        .context("Failed to read awesome-list content")?;

    Ok(extract_github_urls(&markdown))
}

/// Load and parse all seeds from a configuration
pub async fn load_all_seeds(config: &SeedConfig) -> Result<Vec<(String, String)>> {
    let mut all_repos = HashSet::new();

    for seed in &config.seeds {
        match seed.seed_type.as_str() {
            "awesome-list" => {
                tracing::info!(name = %seed.name, url = %seed.url, "Fetching awesome-list");

                match fetch_awesome_list(&seed.url).await {
                    Ok(repos) => {
                        tracing::info!(
                            name = %seed.name,
                            count = repos.len(),
                            "Extracted repositories from awesome-list"
                        );
                        all_repos.extend(repos);
                    }
                    Err(e) => {
                        tracing::warn!(
                            name = %seed.name,
                            error = %e,
                            "Failed to fetch awesome-list, skipping"
                        );
                    }
                }
            }
            "file" => {
                let content = std::fs::read_to_string(&seed.url)
                    .with_context(|| format!("Failed to read seed file: {}", seed.url))?;
                let repos = extract_github_urls(&content);
                tracing::info!(
                    name = %seed.name,
                    count = repos.len(),
                    "Extracted repositories from file"
                );
                all_repos.extend(repos);
            }
            _ => {
                tracing::warn!(
                    name = %seed.name,
                    seed_type = %seed.seed_type,
                    "Unknown seed type, skipping"
                );
            }
        }
    }

    Ok(all_repos.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_github_urls_from_markdown() {
        let markdown = r#"
# Awesome Rust

## CLI Tools

- [ripgrep](https://github.com/BurntSushi/ripgrep) - Fast grep
- [fd](https://github.com/sharkdp/fd) - Fast find
- [bat](https://github.com/sharkdp/bat) - Cat with wings

## Not GitHub

- [Some Tool](https://gitlab.com/user/repo) - GitLab link
"#;

        let urls = extract_github_urls(markdown);
        assert_eq!(urls.len(), 3);

        let urls_set: HashSet<_> = urls.into_iter().collect();
        assert!(urls_set.contains(&("BurntSushi".to_string(), "ripgrep".to_string())));
        assert!(urls_set.contains(&("sharkdp".to_string(), "fd".to_string())));
        assert!(urls_set.contains(&("sharkdp".to_string(), "bat".to_string())));
    }

    #[test]
    fn test_extract_handles_duplicates() {
        let markdown = r#"
- [ripgrep](https://github.com/BurntSushi/ripgrep)
- [ripgrep again](https://github.com/BurntSushi/ripgrep)
"#;

        let urls = extract_github_urls(markdown);
        assert_eq!(urls.len(), 1);
    }

    #[test]
    fn test_load_seed_config() {
        let toml = r#"
[[seeds]]
name = "Awesome Rust"
type = "awesome-list"
url = "https://raw.githubusercontent.com/rust-unofficial/awesome-rust/main/README.md"

[[queries]]
name = "Popular Rust"
query = "language:rust stars:>100"
limit = 500
"#;

        let config: SeedConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.seeds.len(), 1);
        assert_eq!(config.queries.len(), 1);
        assert_eq!(config.queries[0].limit, 500);
    }
}
