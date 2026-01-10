//! Site generator - creates static HTML pages from repository data

use anyhow::{Context, Result};
use console::style;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tera::{Context as TeraContext, Tera};

use crate::models::Repository;

/// Configuration for site generation
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    /// Output directory
    pub output_dir: PathBuf,
    /// Site title
    pub title: String,
    /// Site description
    pub description: String,
    /// Base URL for the site
    pub base_url: String,
    /// Whether to generate JSON data files
    pub generate_json: bool,
    /// Whether to generate language pages
    pub generate_language_pages: bool,
    /// Whether to generate topic pages
    pub generate_topic_pages: bool,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("./dist"),
            title: "Awesomeify".to_string(),
            description: "Discover and explore open source repositories".to_string(),
            base_url: "/".to_string(),
            generate_json: true,
            generate_language_pages: true,
            generate_topic_pages: true,
        }
    }
}

/// Statistics about the generated site
#[derive(Debug)]
pub struct GeneratorStats {
    pub total_repos: usize,
    pub languages: Vec<LanguageStats>,
    pub topics: Vec<TopicStats>,
    pub total_stars: u64,
    pub active_repos: usize,
}

#[derive(Debug, Clone)]
pub struct LanguageStats {
    pub name: String,
    pub count: usize,
    pub slug: String,
}

#[derive(Debug, Clone)]
pub struct TopicStats {
    pub name: String,
    pub count: usize,
    pub slug: String,
}

/// Site generator that creates static HTML pages
pub struct SiteGenerator {
    tera: Tera,
    config: GeneratorConfig,
}

impl SiteGenerator {
    /// Create a new site generator with embedded templates
    pub fn new(config: GeneratorConfig) -> Result<Self> {
        let mut tera = Tera::default();

        // Add embedded templates
        tera.add_raw_templates(vec![
            ("base.html", include_str!("../../templates/base.html")),
            ("index.html", include_str!("../../templates/index.html")),
            ("category.html", include_str!("../../templates/category.html")),
            ("macros.html", include_str!("../../templates/macros.html")),
        ])?;

        // Add custom filters
        tera.register_filter("format_number", format_number_filter);
        tera.register_filter("format_date", format_date_filter);
        tera.register_filter("slugify", slugify_filter);

        Ok(Self { tera, config })
    }

    /// Generate the complete static site
    pub fn generate(&self, repos: &[Repository]) -> Result<GeneratorStats> {
        let output_dir = &self.config.output_dir;

        // Create output directory
        std::fs::create_dir_all(output_dir)
            .with_context(|| format!("Failed to create output directory: {}", output_dir.display()))?;

        println!(
            "   {} Generating site to {}",
            style("→").dim(),
            style(output_dir.display()).yellow()
        );

        // Calculate statistics
        let stats = self.calculate_stats(repos);

        // Generate JSON data files
        if self.config.generate_json {
            self.generate_json_files(repos, output_dir)?;
        }

        // Generate HTML pages
        self.generate_index_page(repos, &stats, output_dir)?;

        if self.config.generate_language_pages {
            self.generate_language_pages(repos, &stats, output_dir)?;
        }

        if self.config.generate_topic_pages {
            self.generate_topic_pages(repos, output_dir)?;
        }

        // Copy static assets
        self.copy_static_assets(output_dir)?;

        Ok(stats)
    }

    /// Calculate statistics from repositories
    fn calculate_stats(&self, repos: &[Repository]) -> GeneratorStats {
        let mut language_counts: HashMap<String, usize> = HashMap::new();
        let mut topic_counts: HashMap<String, usize> = HashMap::new();
        let mut total_stars: u64 = 0;
        let mut active_repos = 0;

        for repo in repos {
            total_stars += repo.stars as u64;

            if repo.activity_score >= 50.0 {
                active_repos += 1;
            }

            if let Some(lang) = &repo.language {
                *language_counts.entry(lang.clone()).or_default() += 1;
            }

            for topic in &repo.topics {
                *topic_counts.entry(topic.clone()).or_default() += 1;
            }
        }

        let mut languages: Vec<LanguageStats> = language_counts
            .into_iter()
            .map(|(name, count)| LanguageStats {
                slug: slugify(&name),
                name,
                count,
            })
            .collect();
        languages.sort_by(|a, b| b.count.cmp(&a.count));

        let mut topics: Vec<TopicStats> = topic_counts
            .into_iter()
            .map(|(name, count)| TopicStats {
                slug: slugify(&name),
                name,
                count,
            })
            .collect();
        topics.sort_by(|a, b| b.count.cmp(&a.count));

        GeneratorStats {
            total_repos: repos.len(),
            languages,
            topics,
            total_stars,
            active_repos,
        }
    }

    /// Generate JSON data files
    fn generate_json_files(&self, repos: &[Repository], output_dir: &Path) -> Result<()> {
        let data_dir = output_dir.join("data");
        std::fs::create_dir_all(&data_dir)?;

        // Full dataset
        let json = serde_json::to_string_pretty(repos)?;
        std::fs::write(data_dir.join("repos.json"), json)?;

        // By language
        let by_lang_dir = data_dir.join("by-language");
        std::fs::create_dir_all(&by_lang_dir)?;

        let mut by_language: HashMap<String, Vec<&Repository>> = HashMap::new();
        for repo in repos {
            if let Some(lang) = &repo.language {
                by_language.entry(slugify(lang)).or_default().push(repo);
            }
        }

        for (slug, lang_repos) in &by_language {
            let json = serde_json::to_string_pretty(lang_repos)?;
            std::fs::write(by_lang_dir.join(format!("{}.json", slug)), json)?;
        }

        // By topic
        let by_topic_dir = data_dir.join("by-topic");
        std::fs::create_dir_all(&by_topic_dir)?;

        let mut by_topic: HashMap<String, Vec<&Repository>> = HashMap::new();
        for repo in repos {
            for topic in &repo.topics {
                by_topic.entry(slugify(topic)).or_default().push(repo);
            }
        }

        for (slug, topic_repos) in &by_topic {
            let json = serde_json::to_string_pretty(topic_repos)?;
            std::fs::write(by_topic_dir.join(format!("{}.json", slug)), json)?;
        }

        println!(
            "   {} Generated JSON data files",
            style("✓").green()
        );

        Ok(())
    }

    /// Generate the main index page
    fn generate_index_page(
        &self,
        repos: &[Repository],
        stats: &GeneratorStats,
        output_dir: &Path,
    ) -> Result<()> {
        let mut context = TeraContext::new();

        // Site config
        context.insert("title", &self.config.title);
        context.insert("description", &self.config.description);
        context.insert("base_url", &self.config.base_url);

        // Repository data (limit for initial page load)
        let display_repos: Vec<&Repository> = repos.iter().take(100).collect();
        context.insert("repos", &display_repos);
        context.insert("total_repos", &repos.len());

        // Statistics
        context.insert("total_count", &stats.total_repos);
        context.insert("language_count", &stats.languages.len());
        context.insert("topic_count", &stats.topics.len());
        context.insert("total_stars", &stats.total_stars);
        context.insert("active_count", &stats.active_repos);

        // Top languages and topics for navigation
        let top_languages: Vec<&LanguageStats> = stats.languages.iter().take(15).collect();
        context.insert("languages", &top_languages);

        let top_topics: Vec<&TopicStats> = stats.topics.iter().take(20).collect();
        context.insert("topics", &top_topics);

        let html = self.tera.render("index.html", &context)?;
        std::fs::write(output_dir.join("index.html"), html)?;

        println!(
            "   {} Generated index.html",
            style("✓").green()
        );

        Ok(())
    }

    /// Generate language category pages
    fn generate_language_pages(
        &self,
        repos: &[Repository],
        stats: &GeneratorStats,
        output_dir: &Path,
    ) -> Result<()> {
        let lang_dir = output_dir.join("language");
        std::fs::create_dir_all(&lang_dir)?;

        // Group repos by language
        let mut by_language: HashMap<String, Vec<&Repository>> = HashMap::new();
        for repo in repos {
            if let Some(lang) = &repo.language {
                by_language.entry(lang.clone()).or_default().push(repo);
            }
        }

        for (language, mut lang_repos) in by_language {
            // Sort by stars
            lang_repos.sort_by(|a, b| b.stars.cmp(&a.stars));

            let mut context = TeraContext::new();
            context.insert("title", &format!("{} - {}", language, self.config.title));
            context.insert("base_url", &self.config.base_url);
            context.insert("category_type", "language");
            context.insert("category_name", &language);
            context.insert("category_slug", &slugify(&language));
            context.insert("repos", &lang_repos);
            context.insert("count", &lang_repos.len());

            // Add navigation
            let top_languages: Vec<&LanguageStats> = stats.languages.iter().take(15).collect();
            context.insert("languages", &top_languages);

            let html = self.tera.render("category.html", &context)?;
            std::fs::write(lang_dir.join(format!("{}.html", slugify(&language))), html)?;
        }

        println!(
            "   {} Generated {} language pages",
            style("✓").green(),
            stats.languages.len()
        );

        Ok(())
    }

    /// Generate topic category pages
    fn generate_topic_pages(&self, repos: &[Repository], output_dir: &Path) -> Result<()> {
        let topic_dir = output_dir.join("topic");
        std::fs::create_dir_all(&topic_dir)?;

        // Group repos by topic
        let mut by_topic: HashMap<String, Vec<&Repository>> = HashMap::new();
        for repo in repos {
            for topic in &repo.topics {
                by_topic.entry(topic.clone()).or_default().push(repo);
            }
        }

        let topic_count = by_topic.len();

        for (topic, mut topic_repos) in by_topic {
            // Sort by stars
            topic_repos.sort_by(|a, b| b.stars.cmp(&a.stars));

            let mut context = TeraContext::new();
            context.insert("title", &format!("{} - {}", topic, self.config.title));
            context.insert("base_url", &self.config.base_url);
            context.insert("category_type", "topic");
            context.insert("category_name", &topic);
            context.insert("category_slug", &slugify(&topic));
            context.insert("repos", &topic_repos);
            context.insert("count", &topic_repos.len());

            let html = self.tera.render("category.html", &context)?;
            std::fs::write(topic_dir.join(format!("{}.html", slugify(&topic))), html)?;
        }

        println!(
            "   {} Generated {} topic pages",
            style("✓").green(),
            topic_count
        );

        Ok(())
    }

    /// Copy static assets to output directory
    fn copy_static_assets(&self, output_dir: &Path) -> Result<()> {
        // Create directories
        let css_dir = output_dir.join("css");
        let js_dir = output_dir.join("js");
        std::fs::create_dir_all(&css_dir)?;
        std::fs::create_dir_all(&js_dir)?;

        // Write embedded CSS
        std::fs::write(
            css_dir.join("style.css"),
            include_str!("../../site/css/style.css"),
        )?;

        // Write embedded JS
        std::fs::write(
            js_dir.join("app.js"),
            include_str!("../../site/js/app.js"),
        )?;

        println!(
            "   {} Copied static assets",
            style("✓").green()
        );

        Ok(())
    }
}

/// Slugify a string for use in URLs
fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else if c == ' ' || c == '_' || c == '-' {
                '-'
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Tera filter to format numbers with commas
fn format_number_filter(
    value: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    let n = value
        .as_u64()
        .or_else(|| value.as_i64().map(|i| i as u64))
        .unwrap_or(0);

    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, c);
    }

    Ok(tera::Value::String(result))
}

/// Tera filter to format dates
fn format_date_filter(
    value: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    if let Some(s) = value.as_str() {
        // Try to parse and format the date
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
            return Ok(tera::Value::String(dt.format("%b %d, %Y").to_string()));
        }
    }
    Ok(value.clone())
}

/// Tera filter to slugify strings
fn slugify_filter(
    value: &tera::Value,
    _args: &HashMap<String, tera::Value>,
) -> tera::Result<tera::Value> {
    if let Some(s) = value.as_str() {
        return Ok(tera::Value::String(slugify(s)));
    }
    Ok(value.clone())
}
