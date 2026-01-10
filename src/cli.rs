use std::path::PathBuf;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "awesomeify")]
#[command(about = "Index and search open source repositories to create awesome lists on demand")]
#[command(version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Index repositories from configured sources
    Index(IndexArgs),

    /// Generate static site from indexed data
    Generate(GenerateArgs),

    /// Start local development server
    Serve(ServeArgs),

    /// Show database statistics
    Stats,
}

#[derive(Parser)]
pub struct IndexArgs {
    /// GitHub search query (e.g., "stars:>100 language:rust")
    #[arg(short, long)]
    pub query: Option<String>,

    /// Path to seed file (awesome-list URLs)
    #[arg(short, long)]
    pub seed: Option<PathBuf>,

    /// Maximum number of repositories to index
    #[arg(short, long, default_value = "1000")]
    pub limit: usize,

    /// Source to index from
    #[arg(long, default_value = "github")]
    pub source: String,

    /// Force re-index even if recently updated
    #[arg(long)]
    pub force: bool,
}

#[derive(Parser)]
pub struct GenerateArgs {
    /// Output directory for generated site
    #[arg(short, long, default_value = "./dist")]
    pub output: PathBuf,

    /// Generate JSON data chunks by category
    #[arg(long, default_value = "true")]
    pub chunk_data: bool,

    /// Minimum stars to include in output
    #[arg(long, default_value = "0")]
    pub min_stars: u32,

    /// Only include active repositories (activity score > threshold)
    #[arg(long)]
    pub active_only: bool,
}

#[derive(Parser)]
pub struct ServeArgs {
    /// Port to serve on
    #[arg(short, long, default_value = "3000")]
    pub port: u16,

    /// Directory to serve
    #[arg(short, long, default_value = "./dist")]
    pub dir: PathBuf,
}
