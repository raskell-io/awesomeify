-- Initial schema for awesomeify

CREATE TABLE IF NOT EXISTS repositories (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL,
    full_name TEXT NOT NULL,
    url TEXT NOT NULL,
    description TEXT,
    topics TEXT, -- JSON array
    language TEXT,
    languages TEXT, -- JSON object
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
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_repos_source ON repositories(source);
CREATE INDEX IF NOT EXISTS idx_repos_language ON repositories(language);
CREATE INDEX IF NOT EXISTS idx_repos_stars ON repositories(stars DESC);
CREATE INDEX IF NOT EXISTS idx_repos_activity ON repositories(activity_score DESC);
CREATE INDEX IF NOT EXISTS idx_repos_pushed ON repositories(pushed_at DESC);

-- Full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS repositories_fts USING fts5(
    full_name,
    description,
    topics,
    content=repositories,
    content_rowid=rowid
);

-- Triggers to keep FTS in sync
CREATE TRIGGER IF NOT EXISTS repos_ai AFTER INSERT ON repositories BEGIN
    INSERT INTO repositories_fts(rowid, full_name, description, topics)
    VALUES (NEW.rowid, NEW.full_name, NEW.description, NEW.topics);
END;

CREATE TRIGGER IF NOT EXISTS repos_ad AFTER DELETE ON repositories BEGIN
    INSERT INTO repositories_fts(repositories_fts, rowid, full_name, description, topics)
    VALUES('delete', OLD.rowid, OLD.full_name, OLD.description, OLD.topics);
END;

CREATE TRIGGER IF NOT EXISTS repos_au AFTER UPDATE ON repositories BEGIN
    INSERT INTO repositories_fts(repositories_fts, rowid, full_name, description, topics)
    VALUES('delete', OLD.rowid, OLD.full_name, OLD.description, OLD.topics);
    INSERT INTO repositories_fts(rowid, full_name, description, topics)
    VALUES (NEW.rowid, NEW.full_name, NEW.description, NEW.topics);
END;
