CREATE TABLE IF NOT EXISTS commit_files (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    commit_id UUID NOT NULL REFERENCES commits(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,
    status TEXT NOT NULL,
    additions INT NOT NULL DEFAULT 0,
    deletions INT NOT NULL DEFAULT 0,
    old_path TEXT
);
CREATE INDEX IF NOT EXISTS idx_commit_files_commit ON commit_files(commit_id);
CREATE INDEX IF NOT EXISTS idx_commit_files_path ON commit_files(file_path);
