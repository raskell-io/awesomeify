# Awesomeify

A Rust CLI tool that indexes open source repositories from GitHub (and other forges) and generates static, searchable "Awesome List" style pages.

## Project Overview

Awesomeify solves the problem of discovering and organizing open source projects by:
1. Indexing repositories from multiple sources (GitHub, GitLab, Codeberg)
2. Scoring activity and health metrics
3. Generating fully static, filterable web pages
4. Enabling custom "awesome list" views on demand

## Architecture

```
Sources (GitHub API, etc.)
    ↓
Indexer (Rust CLI)
    ↓
SQLite Database
    ↓
Static Site Generator
    ↓
HTML + JSON + Pagefind Index
```

## Tech Stack

- **Language**: Rust
- **Database**: SQLite (via sqlx)
- **APIs**: GitHub GraphQL, GitHub Search API
- **Templating**: Tera
- **Search**: Pagefind (static WASM-based search)
- **Task Runner**: mise

## Key Commands

```bash
mise run build        # Build release binary
mise run index        # Index repositories
mise run generate     # Generate static site
mise run pagefind     # Build search index
mise run full         # Full pipeline
mise run serve        # Local dev server
```

## Code Conventions

- Use `thiserror` for error types
- Use `tracing` for logging (not `log`)
- Async everywhere with `tokio`
- Prefer `anyhow::Result` in binaries, typed errors in library code
- Keep modules focused: one responsibility per file
- Write integration tests for API clients using recorded fixtures

## Directory Structure

```
src/
├── main.rs              # CLI entry, clap setup
├── cli.rs               # Command definitions
├── sources/             # API clients (GitHub, etc.)
├── models/              # Data structures
├── storage/             # SQLite operations
├── activity/            # Activity scoring
└── generator/           # Static site output
```

## Environment Variables

- `GITHUB_TOKEN` - GitHub personal access token (required for API)
- `RUST_LOG` - Logging level (default: awesomeify=debug)
- `DATABASE_URL` - SQLite database path (default: ./data/awesomeify.db)
