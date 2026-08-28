CREATE TABLE IF NOT EXISTS symbol_dependencies (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    symbol_id UUID NOT NULL REFERENCES symbols(id) ON DELETE CASCADE,
    depends_on_symbol_id UUID REFERENCES symbols(id) ON DELETE SET NULL,
    dependency_name TEXT NOT NULL,
    dependency_type TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_symbol_deps_symbol ON symbol_dependencies(symbol_id);
CREATE INDEX IF NOT EXISTS idx_symbol_deps_depends ON symbol_dependencies(depends_on_symbol_id);
