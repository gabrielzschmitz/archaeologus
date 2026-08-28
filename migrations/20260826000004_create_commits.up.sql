CREATE TABLE IF NOT EXISTS commits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repository_id UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    sha TEXT NOT NULL,
    author_name TEXT,
    author_email TEXT,
    author_date TIMESTAMPTZ NOT NULL,
    committer_name TEXT,
    committer_email TEXT,
    committer_date TIMESTAMPTZ NOT NULL,
    message TEXT NOT NULL,
    parent_shas TEXT[] NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(repository_id, sha)
);
CREATE INDEX IF NOT EXISTS idx_commits_repo ON commits(repository_id);
CREATE INDEX IF NOT EXISTS idx_commits_author_date ON commits(author_date);
CREATE INDEX IF NOT EXISTS idx_commits_sha ON commits(sha);
