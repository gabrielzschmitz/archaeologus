CREATE TABLE IF NOT EXISTS symbol_commits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    symbol_id UUID NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
    commit_id UUID NOT NULL REFERENCES commits(id) ON DELETE CASCADE,
    change_type TEXT NOT NULL,
    UNIQUE(symbol_id, commit_id)
);
CREATE INDEX IF NOT EXISTS idx_symbol_commits_symbol ON symbol_commits(symbol_id);
CREATE INDEX IF NOT EXISTS idx_symbol_commits_commit ON symbol_commits(commit_id);
