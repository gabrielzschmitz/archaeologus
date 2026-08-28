CREATE EXTENSION IF NOT EXISTS pg_trgm;

CREATE TABLE IF NOT EXISTS symbols (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    file_id UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    repository_id UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    symbol_type TEXT NOT NULL,
    language TEXT NOT NULL,
    line_start INT NOT NULL,
    line_end INT NOT NULL,
    col_start INT NOT NULL,
    col_end INT NOT NULL,
    visibility TEXT,
    doc_comment TEXT,
    raw_text TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_symbols_repo ON symbols(repository_id);
CREATE INDEX IF NOT EXISTS idx_symbols_file ON symbols(file_id);
CREATE INDEX IF NOT EXISTS idx_symbols_name ON symbols(name);
CREATE INDEX IF NOT EXISTS idx_symbols_type ON symbols(symbol_type);
CREATE INDEX IF NOT EXISTS idx_symbols_name_trgm ON symbols USING gin(name gin_trgm_ops);
