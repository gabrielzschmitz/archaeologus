-- Add unique constraint so upsert_symbol (ON CONFLICT) works correctly.
CREATE UNIQUE INDEX IF NOT EXISTS idx_symbols_file_name_type
    ON symbols (file_id, name, symbol_type);
