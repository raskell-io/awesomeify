use anyhow::{Context, Result};
use clap::Parser;
use console::style;
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod activity;
mod cli;
mod generator;
mod indexer;
mod models;
mod sources;
mod storage;

use cli::{Cli, Commands};
use indexer::{Indexer, IndexerConfig};
use storage::Database;

/// Default database path
fn default_db_path() -> PathBuf {
    PathBuf::from("./data/awesomeify.db")
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "awesomeify=info".into()))
        .with(tracing_subscriber::fmt::layer().without_time())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Index(args) => {
            run_index(args).await?;
        }
        Commands::Generate(args) => {
            run_generate(args).await?;
        }
        Commands::Serve(args) => {
            run_serve(args).await?;
        }
        Commands::Stats => {
            run_stats().await?;
        }
    }

    Ok(())
}

/// Run the index command
async fn run_index(args: cli::IndexArgs) -> Result<()> {
    // Validate arguments
    if args.query.is_none() && args.seed.is_none() {
        eprintln!(
            "{} No search query or seed file provided.",
            style("Error:").red().bold()
        );
        eprintln!();
        eprintln!("Usage examples:");
        eprintln!(
            "  {} awesomeify index --query \"language:rust stars:>100\"",
            style("$").dim()
        );
        eprintln!(
            "  {} awesomeify index --seed seeds/awesome-lists.toml",
            style("$").dim()
        );
        eprintln!(
            "  {} awesomeify index --query \"topic:cli\" --limit 500",
            style("$").dim()
        );
        std::process::exit(1);
    }

    // Check for GitHub token
    if std::env::var("GITHUB_TOKEN").is_err() {
        eprintln!(
            "{} GITHUB_TOKEN environment variable not set.",
            style("Error:").red().bold()
        );
        eprintln!();
        eprintln!("To use the GitHub API, you need a personal access token:");
        eprintln!("  1. Go to https://github.com/settings/tokens");
        eprintln!("  2. Generate a new token with 'public_repo' scope");
        eprintln!("  3. Export it: export GITHUB_TOKEN=ghp_xxxxx");
        std::process::exit(1);
    }

    // Create indexer
    let db_path = default_db_path();
    let indexer = Indexer::new(&db_path)
        .await
        .context("Failed to initialize indexer")?;

    // Build configuration
    let config = IndexerConfig {
        query: args.query,
        seed_file: args.seed,
        limit: args.limit,
        force: args.force,
        fetch_commits: false,
    };

    // Run indexer
    indexer.run(config).await?;

    Ok(())
}

/// Run the generate command
async fn run_generate(args: cli::GenerateArgs) -> Result<()> {
    println!(
        "{} Generating static site...",
        style("▶").cyan().bold()
    );

    let db_path = default_db_path();

    // Check if database exists
    if !db_path.exists() {
        eprintln!(
            "{} Database not found at {}",
            style("Error:").red().bold(),
            db_path.display()
        );
        eprintln!();
        eprintln!("Run 'awesomeify index' first to index some repositories.");
        std::process::exit(1);
    }

    // Open database
    let db = Database::create(&db_path).await?;

    // Get repositories
    let min_activity = if args.active_only { Some(50.0) } else { None };
    let repos = db
        .get_repositories(None, Some(args.min_stars), min_activity, None)
        .await?;

    if repos.is_empty() {
        eprintln!(
            "{} No repositories found matching filters.",
            style("Warning:").yellow().bold()
        );
        return Ok(());
    }

    println!(
        "   Found {} repositories",
        style(repos.len()).cyan()
    );

    // Create site generator
    let config = generator::GeneratorConfig {
        output_dir: args.output.clone(),
        title: "Awesomeify".to_string(),
        description: "Discover and explore open source repositories".to_string(),
        base_url: "/".to_string(),
        generate_json: true,
        generate_language_pages: true,
        generate_topic_pages: true,
    };

    let site_generator = generator::SiteGenerator::new(config)
        .context("Failed to initialize site generator")?;

    // Generate site
    let stats = site_generator.generate(&repos)?;

    println!();
    println!(
        "{} Site generated successfully!",
        style("✓").green().bold()
    );
    println!(
        "   Output: {}",
        style(args.output.display()).yellow()
    );
    println!(
        "   {} repos, {} languages, {} topics",
        style(stats.total_repos).cyan(),
        style(stats.languages.len()).cyan(),
        style(stats.topics.len()).cyan()
    );

    println!();
    println!("Next steps:");
    println!(
        "  {} npx pagefind --site {} --bundle-dir pagefind",
        style("1.").dim(),
        args.output.display()
    );
    println!(
        "  {} npx serve {}",
        style("2.").dim(),
        args.output.display()
    );

    Ok(())
}

/// Run the serve command
async fn run_serve(args: cli::ServeArgs) -> Result<()> {
    println!(
        "{} Starting development server...",
        style("▶").cyan().bold()
    );

    if !args.dir.exists() {
        eprintln!(
            "{} Directory not found: {}",
            style("Error:").red().bold(),
            args.dir.display()
        );
        eprintln!();
        eprintln!("Run 'awesomeify generate' first to generate the static site.");
        std::process::exit(1);
    }

    println!(
        "   Serving {} on http://localhost:{}",
        style(args.dir.display()).yellow(),
        style(args.port).cyan()
    );
    println!();
    println!("   Press Ctrl+C to stop");
    println!();

    // Use a simple HTTP server (or suggest using npx serve)
    println!(
        "{} For a full-featured server, use:",
        style("Tip:").blue().bold()
    );
    println!(
        "  {} npx serve {} -p {}",
        style("$").dim(),
        args.dir.display(),
        args.port
    );

    Ok(())
}

/// Run the stats command
async fn run_stats() -> Result<()> {
    let db_path = default_db_path();

    if !db_path.exists() {
        eprintln!(
            "{} Database not found at {}",
            style("Error:").red().bold(),
            db_path.display()
        );
        eprintln!();
        eprintln!("Run 'awesomeify index' first to index some repositories.");
        std::process::exit(1);
    }

    let db = Database::create(&db_path).await?;
    let stats = db.stats().await?;

    println!();
    println!(
        "{} Database Statistics",
        style("📊").bold()
    );
    println!();
    println!(
        "   Total repositories:  {}",
        style(stats.total_repositories).cyan().bold()
    );
    println!(
        "   Unique languages:    {}",
        style(stats.unique_languages).cyan()
    );
    println!(
        "   Total stars:         {}",
        style(format_number(stats.total_stars)).cyan()
    );
    println!(
        "   Active (score ≥50):  {}",
        style(stats.active_repositories).green()
    );

    if !stats.top_languages.is_empty() {
        println!();
        println!("   Top languages:");
        for (lang, count) in stats.top_languages.iter().take(10) {
            println!(
                "     {}: {}",
                style(format!("{:15}", lang)).yellow(),
                style(count).dim()
            );
        }
    }

    println!();

    Ok(())
}

/// Format a large number with commas
fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.insert(0, ',');
        }
        result.insert(0, c);
    }
    result
}
