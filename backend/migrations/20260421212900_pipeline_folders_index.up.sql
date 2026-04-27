-- Fast lookups for:
--   1. "does folder F have a pipeline?"  (exists check on prefix)
--   2. "list all folders with a pipeline" (distinct folder from path)
-- The partial predicate keeps the index tiny on workspaces with few
-- materializer scripts, and text_pattern_ops lets 'f/foo/%' LIKE scans use it.
CREATE INDEX IF NOT EXISTS idx_script_materializer_path
  ON script (workspace_id, path text_pattern_ops)
  WHERE auto_kind = 'materializer' AND archived = false AND deleted = false;
