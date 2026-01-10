use anyhow::Result;
use std::path::Path;
use crate::models::Repository;

/// Export repositories to JSON files
pub async fn export_json(repos: &[Repository], output_dir: &Path) -> Result<()> {
    // Ensure output directory exists
    std::fs::create_dir_all(output_dir)?;

    // Export full dataset
    let full_path = output_dir.join("repos.json");
    let json = serde_json::to_string_pretty(repos)?;
    std::fs::write(&full_path, json)?;
    tracing::info!("Wrote {} repos to {}", repos.len(), full_path.display());

    // Export by language
    let by_language_dir = output_dir.join("by-language");
    std::fs::create_dir_all(&by_language_dir)?;
    export_by_category(repos, &by_language_dir, |r| {
        r.language.clone().unwrap_or_else(|| "unknown".to_string())
    })?;

    // Export by topic (each repo can have multiple topics)
    let by_topic_dir = output_dir.join("by-topic");
    std::fs::create_dir_all(&by_topic_dir)?;
    export_by_topics(repos, &by_topic_dir)?;

    Ok(())
}

fn export_by_category<F>(repos: &[Repository], dir: &Path, get_category: F) -> Result<()>
where
    F: Fn(&Repository) -> String,
{
    use std::collections::HashMap;

    let mut by_category: HashMap<String, Vec<&Repository>> = HashMap::new();

    for repo in repos {
        let category = get_category(repo).to_lowercase();
        by_category.entry(category).or_default().push(repo);
    }

    for (category, category_repos) in by_category {
        let filename = format!("{}.json", sanitize_filename(&category));
        let path = dir.join(&filename);
        let json = serde_json::to_string_pretty(&category_repos)?;
        std::fs::write(&path, json)?;
        tracing::debug!("Wrote {} repos to {}", category_repos.len(), path.display());
    }

    Ok(())
}

fn export_by_topics(repos: &[Repository], dir: &Path) -> Result<()> {
    use std::collections::HashMap;

    let mut by_topic: HashMap<String, Vec<&Repository>> = HashMap::new();

    for repo in repos {
        for topic in &repo.topics {
            by_topic.entry(topic.to_lowercase()).or_default().push(repo);
        }
    }

    for (topic, topic_repos) in by_topic {
        let filename = format!("{}.json", sanitize_filename(&topic));
        let path = dir.join(&filename);
        let json = serde_json::to_string_pretty(&topic_repos)?;
        std::fs::write(&path, json)?;
        tracing::debug!("Wrote {} repos to {}", topic_repos.len(), path.display());
    }

    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect()
}
