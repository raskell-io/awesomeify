use chrono::Utc;
use crate::models::Repository;

/// Calculate an activity score for a repository (0-100)
///
/// The score is based on:
/// - Recency of last push (40 points max)
/// - Recent commit activity (30 points max)
/// - Open issues indicating active development (10 points max)
/// - Recent releases (20 points max)
pub fn calculate_activity_score(repo: &Repository) -> f32 {
    let now = Utc::now();

    // Recency score: how recently was code pushed?
    // Full points if pushed today, decreasing linearly over a year
    let days_since_push = (now - repo.pushed_at).num_days().max(0) as f32;
    let recency_score = (1.0 - (days_since_push / 365.0).min(1.0)) * 40.0;

    // Commit score: recent commits indicate active development
    // Cap at 50 commits for full points
    let commit_score = (repo.recent_commits_30d as f32).min(50.0) / 50.0 * 30.0;

    // Issue score: open issues often indicate active projects
    // But could also indicate abandonment, so cap the contribution
    let issue_score = if repo.open_issues > 0 && repo.open_issues < 100 {
        10.0
    } else if repo.open_issues >= 100 {
        5.0 // Too many open issues might be a bad sign
    } else {
        0.0
    };

    // Release score: recent releases show active maintenance
    let release_score = repo
        .last_release
        .map(|release_date| {
            let days_since_release = (now - release_date).num_days().max(0) as f32;
            if days_since_release < 30.0 {
                20.0 // Released in last month
            } else if days_since_release < 90.0 {
                15.0 // Released in last quarter
            } else if days_since_release < 180.0 {
                10.0 // Released in last 6 months
            } else if days_since_release < 365.0 {
                5.0 // Released in last year
            } else {
                0.0
            }
        })
        .unwrap_or(0.0);

    // Penalties
    let mut score = recency_score + commit_score + issue_score + release_score;

    // Archived repos get a big penalty
    if repo.archived {
        score *= 0.1;
    }

    // Forks get a small penalty (prefer original repos)
    if repo.is_fork {
        score *= 0.8;
    }

    score.min(100.0).max(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Source;
    use std::collections::HashMap;

    fn make_test_repo() -> Repository {
        Repository {
            id: "github:test/repo".to_string(),
            source: Source::GitHub,
            full_name: "test/repo".to_string(),
            url: "https://github.com/test/repo".to_string(),
            description: Some("Test repo".to_string()),
            topics: vec![],
            language: Some("Rust".to_string()),
            languages: HashMap::new(),
            stars: 100,
            forks: 10,
            watchers: 50,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            pushed_at: Utc::now(),
            open_issues: 5,
            recent_commits_30d: 20,
            last_release: Some(Utc::now()),
            activity_score: 0.0,
            indexed_at: Utc::now(),
            archived: false,
            is_fork: false,
            license: Some("MIT".to_string()),
            homepage: None,
        }
    }

    #[test]
    fn test_active_repo_has_high_score() {
        let repo = make_test_repo();
        let score = calculate_activity_score(&repo);
        assert!(score > 80.0, "Active repo should have high score, got {}", score);
    }

    #[test]
    fn test_archived_repo_has_low_score() {
        let mut repo = make_test_repo();
        repo.archived = true;
        let score = calculate_activity_score(&repo);
        assert!(score < 20.0, "Archived repo should have low score, got {}", score);
    }
}
