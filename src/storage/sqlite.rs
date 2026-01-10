//! SQLite storage for repository data

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::Path;

use crate::models::{Repository, Source};

/// SQLite database for repository storage
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Connect to an existing database
    pub async fn connect(url: &str) -> Result<Self> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .context("Failed to connect to database")?;

        Ok(Self { pool })
    }

    /// Create a new database at the given path
    pub async fn create(path: &Path) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let url = format!("sqlite:{}?mode=rwc", path.display());
        let db = Self::connect(&url).await?;
        db.run_migrations().await?;
        Ok(db)
    }

    /// Run database migrations
    async fn run_migrations(&self) -> Result<()> {
        // Run embedded migrations
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS repositories (
                id TEXT PRIMARY KEY,
                source TEXT NOT NULL,
                full_name TEXT NOT NULL,
                url TEXT NOT NULL,
                description TEXT,
                topics TEXT,
                language TEXT,
                languages TEXT,
                stars INTEGER NOT NULL DEFAULT 0,
                forks INTEGER NOT NULL DEFAULT 0,
                watchers INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                pushed_at TEXT NOT NULL,
                open_issues INTEGER NOT NULL DEFAULT 0,
                recent_commits_30d INTEGER NOT NULL DEFAULT 0,
                last_release TEXT,
                activity_score REAL NOT NULL DEFAULT 0,
                indexed_at TEXT NOT NULL,
                archived INTEGER NOT NULL DEFAULT 0,
                is_fork INTEGER NOT NULL DEFAULT 0,
                license TEXT,
                homepage TEXT
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_repos_source ON repositories(source)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_repos_language ON repositories(language)")
            .execute(&self.pool)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_repos_stars ON repositories(stars DESC)")
            .execute(&self.pool)
            .await?;
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_repos_activity ON repositories(activity_score DESC)",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get the connection pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Insert or update a repository
    pub async fn upsert_repository(&self, repo: &Repository) -> Result<()> {
        let topics_json = serde_json::to_string(&repo.topics)?;
        let languages_json = serde_json::to_string(&repo.languages)?;
        let source_str = match repo.source {
            Source::GitHub => "github",
            Source::GitLab => "gitlab",
            Source::Codeberg => "codeberg",
        };

        sqlx::query(
            r#"
            INSERT INTO repositories (
                id, source, full_name, url, description, topics, language, languages,
                stars, forks, watchers, created_at, updated_at, pushed_at, open_issues,
                recent_commits_30d, last_release, activity_score, indexed_at, archived,
                is_fork, license, homepage
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                ?17, ?18, ?19, ?20, ?21, ?22, ?23
            )
            ON CONFLICT(id) DO UPDATE SET
                source = excluded.source,
                full_name = excluded.full_name,
                url = excluded.url,
                description = excluded.description,
                topics = excluded.topics,
                language = excluded.language,
                languages = excluded.languages,
                stars = excluded.stars,
                forks = excluded.forks,
                watchers = excluded.watchers,
                created_at = excluded.created_at,
                updated_at = excluded.updated_at,
                pushed_at = excluded.pushed_at,
                open_issues = excluded.open_issues,
                recent_commits_30d = excluded.recent_commits_30d,
                last_release = excluded.last_release,
                activity_score = excluded.activity_score,
                indexed_at = excluded.indexed_at,
                archived = excluded.archived,
                is_fork = excluded.is_fork,
                license = excluded.license,
                homepage = excluded.homepage
            "#,
        )
        .bind(&repo.id)
        .bind(source_str)
        .bind(&repo.full_name)
        .bind(&repo.url)
        .bind(&repo.description)
        .bind(&topics_json)
        .bind(&repo.language)
        .bind(&languages_json)
        .bind(repo.stars as i64)
        .bind(repo.forks as i64)
        .bind(repo.watchers as i64)
        .bind(repo.created_at.to_rfc3339())
        .bind(repo.updated_at.to_rfc3339())
        .bind(repo.pushed_at.to_rfc3339())
        .bind(repo.open_issues as i64)
        .bind(repo.recent_commits_30d as i64)
        .bind(repo.last_release.map(|dt| dt.to_rfc3339()))
        .bind(repo.activity_score as f64)
        .bind(repo.indexed_at.to_rfc3339())
        .bind(repo.archived)
        .bind(repo.is_fork)
        .bind(&repo.license)
        .bind(&repo.homepage)
        .execute(&self.pool)
        .await
        .context("Failed to upsert repository")?;

        Ok(())
    }

    /// Insert or update multiple repositories in a batch
    pub async fn upsert_repositories(&self, repos: &[Repository]) -> Result<usize> {
        let mut count = 0;
        for repo in repos {
            self.upsert_repository(repo).await?;
            count += 1;
        }
        Ok(count)
    }

    /// Get a repository by ID
    pub async fn get_repository(&self, id: &str) -> Result<Option<Repository>> {
        let row = sqlx::query("SELECT * FROM repositories WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => Ok(Some(self.row_to_repository(&row)?)),
            None => Ok(None),
        }
    }

    /// Get all repositories
    pub async fn get_all_repositories(&self) -> Result<Vec<Repository>> {
        let rows = sqlx::query("SELECT * FROM repositories ORDER BY stars DESC")
            .fetch_all(&self.pool)
            .await?;

        rows.iter().map(|row| self.row_to_repository(row)).collect()
    }

    /// Get repositories with filters
    pub async fn get_repositories(
        &self,
        language: Option<&str>,
        min_stars: Option<u32>,
        min_activity: Option<f32>,
        limit: Option<usize>,
    ) -> Result<Vec<Repository>> {
        let mut query = String::from("SELECT * FROM repositories WHERE 1=1");
        let mut bindings: Vec<String> = Vec::new();

        if let Some(lang) = language {
            query.push_str(" AND language = ?");
            bindings.push(lang.to_string());
        }

        if let Some(stars) = min_stars {
            query.push_str(&format!(" AND stars >= {}", stars));
        }

        if let Some(activity) = min_activity {
            query.push_str(&format!(" AND activity_score >= {}", activity));
        }

        query.push_str(" ORDER BY stars DESC");

        if let Some(lim) = limit {
            query.push_str(&format!(" LIMIT {}", lim));
        }

        let mut q = sqlx::query(&query);
        for binding in &bindings {
            q = q.bind(binding);
        }

        let rows = q.fetch_all(&self.pool).await?;
        rows.iter().map(|row| self.row_to_repository(row)).collect()
    }

    /// Get repository count
    pub async fn count(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM repositories")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get("count"))
    }

    /// Get statistics
    pub async fn stats(&self) -> Result<DatabaseStats> {
        let total: i64 = sqlx::query("SELECT COUNT(*) as count FROM repositories")
            .fetch_one(&self.pool)
            .await?
            .get("count");

        let languages: i64 =
            sqlx::query("SELECT COUNT(DISTINCT language) as count FROM repositories")
                .fetch_one(&self.pool)
                .await?
                .get("count");

        let total_stars: i64 =
            sqlx::query("SELECT COALESCE(SUM(stars), 0) as total FROM repositories")
                .fetch_one(&self.pool)
                .await?
                .get("total");

        let active: i64 =
            sqlx::query("SELECT COUNT(*) as count FROM repositories WHERE activity_score >= 50")
                .fetch_one(&self.pool)
                .await?
                .get("count");

        let top_languages = sqlx::query(
            r#"
            SELECT language, COUNT(*) as count
            FROM repositories
            WHERE language IS NOT NULL
            GROUP BY language
            ORDER BY count DESC
            LIMIT 10
            "#,
        )
        .fetch_all(&self.pool)
        .await?
        .iter()
        .map(|row| {
            (
                row.get::<String, _>("language"),
                row.get::<i64, _>("count") as usize,
            )
        })
        .collect();

        Ok(DatabaseStats {
            total_repositories: total as usize,
            unique_languages: languages as usize,
            total_stars: total_stars as u64,
            active_repositories: active as usize,
            top_languages,
        })
    }

    /// Check if a repository exists
    pub async fn exists(&self, id: &str) -> Result<bool> {
        let row = sqlx::query("SELECT 1 FROM repositories WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.is_some())
    }

    /// Delete a repository
    pub async fn delete(&self, id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM repositories WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    /// Convert a database row to a Repository
    fn row_to_repository(&self, row: &sqlx::sqlite::SqliteRow) -> Result<Repository> {
        let source_str: String = row.get("source");
        let source = match source_str.as_str() {
            "github" => Source::GitHub,
            "gitlab" => Source::GitLab,
            "codeberg" => Source::Codeberg,
            _ => Source::GitHub,
        };

        let topics_json: String = row.get("topics");
        let topics: Vec<String> = serde_json::from_str(&topics_json).unwrap_or_default();

        let languages_json: String = row.get("languages");
        let languages: std::collections::HashMap<String, u64> =
            serde_json::from_str(&languages_json).unwrap_or_default();

        let created_at: String = row.get("created_at");
        let updated_at: String = row.get("updated_at");
        let pushed_at: String = row.get("pushed_at");
        let indexed_at: String = row.get("indexed_at");
        let last_release: Option<String> = row.get("last_release");

        Ok(Repository {
            id: row.get("id"),
            source,
            full_name: row.get("full_name"),
            url: row.get("url"),
            description: row.get("description"),
            topics,
            language: row.get("language"),
            languages,
            stars: row.get::<i64, _>("stars") as u32,
            forks: row.get::<i64, _>("forks") as u32,
            watchers: row.get::<i64, _>("watchers") as u32,
            created_at: DateTime::parse_from_rfc3339(&created_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            updated_at: DateTime::parse_from_rfc3339(&updated_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            pushed_at: DateTime::parse_from_rfc3339(&pushed_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            open_issues: row.get::<i64, _>("open_issues") as u32,
            recent_commits_30d: row.get::<i64, _>("recent_commits_30d") as u32,
            last_release: last_release.and_then(|s| {
                DateTime::parse_from_rfc3339(&s)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            }),
            activity_score: row.get::<f64, _>("activity_score") as f32,
            indexed_at: DateTime::parse_from_rfc3339(&indexed_at)
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(|_| Utc::now()),
            archived: row.get("archived"),
            is_fork: row.get("is_fork"),
            license: row.get("license"),
            homepage: row.get("homepage"),
        })
    }
}

/// Database statistics
#[derive(Debug)]
pub struct DatabaseStats {
    pub total_repositories: usize,
    pub unique_languages: usize,
    pub total_stars: u64,
    pub active_repositories: usize,
    pub top_languages: Vec<(String, usize)>,
}
