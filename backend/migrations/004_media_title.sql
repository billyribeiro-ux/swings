-- Add human-readable title to media items (separate from filename)
ALTER TABLE media ADD COLUMN IF NOT EXISTS title TEXT;
