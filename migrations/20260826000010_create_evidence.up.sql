CREATE TABLE IF NOT EXISTS evidence (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repository_id UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    evidence_type TEXT NOT NULL,
    source_ref TEXT,
    content TEXT NOT NULL,
    confidence TEXT NOT NULL DEFAULT 'UNKNOWN',
    symbol_id UUID REFERENCES symbols(id) ON DELETE SET NULL,
    commit_id UUID REFERENCES commits(id) ON DELETE SET NULL,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_evidence_repo ON evidence(repository_id);
CREATE INDEX IF NOT EXISTS idx_evidence_symbol ON evidence(symbol_id);
CREATE INDEX IF NOT EXISTS idx_evidence_type ON evidence(evidence_type);
