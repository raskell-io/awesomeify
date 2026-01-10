//! Repository indexer - orchestrates fetching and storing repositories

use anyhow::{Context, Result};
use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::path::Path;

use crate::activity::calculate_activity_score;
use crate::sources::{GitHubClient, RepositorySource, SeedConfig};
use crate::storage::Database;

/// Configuration for the indexer
pub struct IndexerConfig {
    /// GitHub search query
    pub query: Option<String>,
    /// Path to seed configuration file
    pub seed_file: Option<std::path::PathBuf>,
    /// Maximum repositories to fetch
    pub limit: usize,
    /// Force re-index even if recently updated
    pub force: bool,
    /// Fetch detailed commit activity (slower but more accurate)
    pub fetch_commits: bool,
}

impl Default for IndexerConfig {
    fn default() -> Self {
        Self {
            query: None,
            seed_file: None,
            limit: 1000,
            force: false,
            fetch_commits: false,
        }
    }
}

/// Result of an indexing run
#[derive(Debug)]
pub struct IndexResult {
    pub fetched: usize,
    pub stored: usize,
    pub skipped: usize,
    pub errors: usize,
}

/// The main indexer that coordinates fetching and storing repositories
pub struct Indexer {
    db: Database,
    github: GitHubClient,
    multi_progress: MultiProgress,
}

impl Indexer {
    /// Create a new indexer
    pub async fn new(db_path: &Path) -> Result<Self> {
        let db = Database::create(db_path)
            .await
            .context("Failed to create/open database")?;

        let github = GitHubClient::from_env().context(
            "Failed to create GitHub client. Make sure GITHUB_TOKEN is set.",
        )?;

        Ok(Self {
            db,
            github,
            multi_progress: MultiProgress::new(),
        })
    }

    /// Create with existing database and client
    pub fn with_components(db: Database, github: GitHubClient) -> Self {
        Self {
            db,
            github,
            multi_progress: MultiProgress::new(),
        }
    }

    /// Run the indexer with the given configuration
    pub async fn run(&self, config: IndexerConfig) -> Result<IndexResult> {
        let mut result = IndexResult {
            fetched: 0,
            stored: 0,
            skipped: 0,
            errors: 0,
        };

        println!(
            "{} Starting indexer...",
            style("▶").cyan().bold()
        );

        // Index from search query if provided
        if let Some(query) = &config.query {
            println!(
                "{} Searching GitHub: {}",
                style("⬇").blue(),
                style(query).yellow()
            );

            match self.index_from_search(query, config.limit, config.force).await {
                Ok(search_result) => {
                    result.fetched += search_result.fetched;
                    result.stored += search_result.stored;
                    result.skipped += search_result.skipped;
                    result.errors += search_result.errors;
                }
                Err(e) => {
                    eprintln!(
                        "{} Search failed: {}",
                        style("✗").red(),
                        e
                    );
                    result.errors += 1;
                }
            }
        }

        // Index from seed file if provided
        if let Some(seed_path) = &config.seed_file {
            println!(
                "{} Loading seeds from: {}",
                style("⬇").blue(),
                style(seed_path.display()).yellow()
            );

            match self.index_from_seeds(seed_path, config.limit, config.force).await {
                Ok(seed_result) => {
                    result.fetched += seed_result.fetched;
                    result.stored += seed_result.stored;
                    result.skipped += seed_result.skipped;
                    result.errors += seed_result.errors;
                }
                Err(e) => {
                    eprintln!(
                        "{} Seed indexing failed: {}",
                        style("✗").red(),
                        e
                    );
                    result.errors += 1;
                }
            }
        }

        // Print summary
        println!();
        println!(
            "{} Indexing complete!",
            style("✓").green().bold()
        );
        println!(
            "   {} fetched, {} stored, {} skipped, {} errors",
            style(result.fetched).cyan(),
            style(result.stored).green(),
            style(result.skipped).yellow(),
            style(result.errors).red()
        );

        // Show database stats
        if let Ok(stats) = self.db.stats().await {
            println!();
            println!(
                "   Database: {} repositories, {} languages, {} total stars",
                style(stats.total_repositories).cyan(),
                style(stats.unique_languages).cyan(),
                style(stats.total_stars).cyan()
            );
        }

        Ok(result)
    }

    /// Index repositories from a GitHub search query
    async fn index_from_search(
        &self,
        query: &str,
        limit: usize,
        force: bool,
    ) -> Result<IndexResult> {
        let mut result = IndexResult {
            fetched: 0,
            stored: 0,
            skipped: 0,
            errors: 0,
        };

        // Create progress bar
        let pb = self.multi_progress.add(ProgressBar::new(limit as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("█▓░"),
        );

        // Create a callback to update progress
        let pb_clone = pb.clone();
        let progress_callback = Box::new(move |fetched: usize, _total: usize| {
            pb_clone.set_position(fetched as u64);
        });

        // Fetch repositories
        let repos = self
            .github
            .search_all(query, limit, Some(progress_callback))
            .await
            .context("GitHub search failed")?;

        pb.finish_and_clear();
        result.fetched = repos.len();

        // Process and store repositories
        let store_pb = self.multi_progress.add(ProgressBar::new(repos.len() as u64));
        store_pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} Storing [{bar:40.cyan/blue}] {pos}/{len}")
                .unwrap()
                .progress_chars("█▓░"),
        );

        for repo in repos {
            store_pb.inc(1);

            // Skip if already indexed and not forcing
            if !force {
                if let Ok(true) = self.db.exists(&repo.id).await {
                    result.skipped += 1;
                    continue;
                }
            }

            // Calculate activity score
            let mut repo = repo;
            repo.activity_score = calculate_activity_score(&repo);

            // Store in database
            match self.db.upsert_repository(&repo).await {
                Ok(_) => result.stored += 1,
                Err(e) => {
                    tracing::warn!(repo = %repo.full_name, error = %e, "Failed to store repository");
                    result.errors += 1;
                }
            }
        }

        store_pb.finish_and_clear();
        Ok(result)
    }

    /// Index repositories from a seed configuration file
    async fn index_from_seeds(
        &self,
        seed_path: &Path,
        limit: usize,
        force: bool,
    ) -> Result<IndexResult> {
        let mut result = IndexResult {
            fetched: 0,
            stored: 0,
            skipped: 0,
            errors: 0,
        };

        // Load seed configuration
        let config = SeedConfig::load(seed_path)?;

        println!(
            "   Found {} seed sources and {} queries",
            style(config.seeds.len()).cyan(),
            style(config.queries.len()).cyan()
        );

        // Process awesome-lists to get repository URLs
        let seed_repos = crate::sources::load_all_seeds(&config).await?;
        println!(
            "   Extracted {} unique repository URLs from seeds",
            style(seed_repos.len()).cyan()
        );

        // Limit the number of repos to fetch
        let repos_to_fetch: Vec<_> = seed_repos.into_iter().take(limit).collect();

        // Create progress bar for fetching
        let pb = self.multi_progress.add(ProgressBar::new(repos_to_fetch.len() as u64));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} Fetching [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("█▓░"),
        );

        // Fetch each repository
        for (owner, name) in &repos_to_fetch {
            pb.inc(1);

            let repo_id = format!("github:{}/{}", owner, name);

            // Skip if already indexed and not forcing
            if !force {
                if let Ok(true) = self.db.exists(&repo_id).await {
                    result.skipped += 1;
                    continue;
                }
            }

            // Fetch repository details
            match self.github.get_repository(owner, name).await {
                Ok(mut repo) => {
                    result.fetched += 1;

                    // Calculate activity score
                    repo.activity_score = calculate_activity_score(&repo);

                    // Store in database
                    match self.db.upsert_repository(&repo).await {
                        Ok(_) => result.stored += 1,
                        Err(e) => {
                            tracing::warn!(repo = %repo.full_name, error = %e, "Failed to store");
                            result.errors += 1;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(repo = %format!("{}/{}", owner, name), error = %e, "Failed to fetch");
                    result.errors += 1;
                }
            }
        }

        pb.finish_and_clear();

        // Also run any configured queries
        for query_config in &config.queries {
            println!(
                "   Running query: {}",
                style(&query_config.name).yellow()
            );

            let query_limit = query_config.limit.min(limit.saturating_sub(result.fetched));
            if query_limit == 0 {
                break;
            }

            match self.index_from_search(&query_config.query, query_limit, force).await {
                Ok(query_result) => {
                    result.fetched += query_result.fetched;
                    result.stored += query_result.stored;
                    result.skipped += query_result.skipped;
                    result.errors += query_result.errors;
                }
                Err(e) => {
                    tracing::warn!(query = %query_config.name, error = %e, "Query failed");
                    result.errors += 1;
                }
            }
        }

        Ok(result)
    }

    /// Get the database reference
    pub fn database(&self) -> &Database {
        &self.db
    }
}
