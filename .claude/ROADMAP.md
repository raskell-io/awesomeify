# Awesomeify Roadmap

## Vision

Create the ultimate tool for discovering, organizing, and sharing curated lists of open source projects. Think "awesome lists as a service" - but fully static and self-hostable.

---

## Phase 1: Foundation

### 1.1 Project Scaffold
- [x] Project structure with mise
- [x] .claude/ configuration
- [ ] Cargo.toml with dependencies
- [ ] Basic CLI structure with clap
- [ ] Error handling setup (thiserror, anyhow)
- [ ] Logging setup (tracing)

### 1.2 Data Model
- [ ] Define `Repository` struct with all fields
- [ ] Define `Source` enum (GitHub, GitLab, Codeberg)
- [ ] Define `IndexConfig` for search queries and seeds
- [ ] Serde serialization for all types

### 1.3 SQLite Storage
- [ ] Database schema design
- [ ] sqlx migrations
- [ ] CRUD operations for repositories
- [ ] Query builders for filtering

---

## Phase 2: GitHub Integration

### 2.1 GitHub GraphQL Client
- [ ] Authentication with GITHUB_TOKEN
- [ ] Rate limit handling with backoff
- [ ] Repository query (single repo metadata)
- [ ] Batch repository queries

### 2.2 GitHub Search API
- [ ] Search query builder
- [ ] Pagination handling
- [ ] Convert search results to Repository model

### 2.3 Seed List Parser
- [ ] Parse awesome-list markdown files
- [ ] Extract repository URLs
- [ ] Deduplicate across sources

### 2.4 Activity Scoring
- [ ] Implement activity score algorithm
- [ ] Fetch commit activity (last 30 days)
- [ ] Fetch release information
- [ ] Calculate composite score

---

## Phase 3: Static Site Generator

### 3.1 JSON Export
- [ ] Full dataset export (repos.json)
- [ ] Chunked exports by language
- [ ] Chunked exports by topic
- [ ] Metadata/stats export

### 3.2 HTML Generation
- [ ] Tera template setup
- [ ] Homepage with search
- [ ] Category landing pages (by language)
- [ ] Topic landing pages
- [ ] Individual repo cards component

### 3.3 Frontend
- [ ] Vanilla JS filtering/sorting
- [ ] Client-side search (pre-Pagefind)
- [ ] Responsive CSS
- [ ] Dark mode support

### 3.4 Pagefind Integration
- [ ] Configure Pagefind for repo data
- [ ] Custom search UI integration
- [ ] Faceted filtering setup

---

## Phase 4: CLI Polish

### 4.1 Commands
- [ ] `awesomeify index` - Run indexer
- [ ] `awesomeify generate` - Generate site
- [ ] `awesomeify serve` - Local preview
- [ ] `awesomeify stats` - Show database stats
- [ ] `awesomeify export` - Export to various formats

### 4.2 Configuration
- [ ] TOML config file support
- [ ] Search query presets
- [ ] Seed list configuration
- [ ] Output customization

### 4.3 Progress & UX
- [ ] Progress bars for long operations
- [ ] Colored output
- [ ] Dry-run mode
- [ ] Verbose logging levels

---

## Phase 5: Additional Sources

### 5.1 Source Abstraction
- [ ] Define `Source` trait
- [ ] Refactor GitHub to implement trait
- [ ] Common rate limiting infrastructure

### 5.2 GitLab Support
- [ ] GitLab API client
- [ ] Public repository discovery
- [ ] Activity metrics

### 5.3 Codeberg Support
- [ ] Codeberg/Gitea API client
- [ ] Repository indexing

### 5.4 SourceHut Support (Optional)
- [ ] SourceHut API exploration
- [ ] Basic integration

---

## Phase 6: Advanced Features

### 6.1 Incremental Updates
- [ ] Track last indexed timestamp
- [ ] Delta updates (only changed repos)
- [ ] Stale repo detection and removal

### 6.2 Custom Views
- [ ] User-defined filter presets
- [ ] "Awesome list" template export
- [ ] Markdown export for GitHub

### 6.3 Enrichment
- [ ] Detect frameworks from package files
- [ ] README analysis for better descriptions
- [ ] License detection and categorization

### 6.4 Analytics
- [ ] Trend detection (rising stars)
- [ ] Ecosystem health metrics
- [ ] Comparison views

---

## Phase 7: Distribution

### 7.1 Packaging
- [ ] GitHub releases with binaries
- [ ] Homebrew formula
- [ ] cargo install support
- [ ] Docker image

### 7.2 CI/CD Templates
- [ ] GitHub Actions workflow for weekly updates
- [ ] Cloudflare Pages deployment
- [ ] Netlify deployment
- [ ] Vercel deployment

### 7.3 Documentation
- [ ] README with examples
- [ ] Configuration reference
- [ ] API documentation
- [ ] Tutorial: "Build your own awesome list"

---

## Future Ideas

- [ ] Browser extension to add repos to your index
- [ ] RSS feed of new discoveries
- [ ] Comparison with other awesome lists
- [ ] Community-contributed seed lists
- [ ] Integration with pkg.go.dev, crates.io, npm for package metadata
- [ ] AI-powered categorization and tagging
- [ ] WebAssembly version for in-browser indexing

---

## Current Focus

**Next up: Phase 1.1 - Complete project scaffold**

Priority tasks:
1. Create Cargo.toml with all dependencies
2. Set up basic CLI with clap
3. Implement tracing/logging
4. Create stub modules for each component
