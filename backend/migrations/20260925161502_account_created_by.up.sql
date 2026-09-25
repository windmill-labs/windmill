-- Who connected an OAuth account. A variable may link an account only if its caller created it
-- or can already read a variable linking it. Existing accounts stay NULL: they can still be
-- linked through a readable variable, never claimed as unlinked.
ALTER TABLE account ADD COLUMN IF NOT EXISTS created_by VARCHAR(255);
