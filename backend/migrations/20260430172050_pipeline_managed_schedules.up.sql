-- Flag schedules auto-created from a pipeline script's `// schedule "<cron>"`
-- annotation so reconciliation can update / drop them on subsequent deploys
-- without touching schedules a user created manually. Defaults to false for
-- pre-existing rows. `script_path` already tells us which script owns the
-- row — this is just a boolean discriminator.
ALTER TABLE schedule ADD COLUMN IF NOT EXISTS managed BOOLEAN NOT NULL DEFAULT false;

-- Partial index for the two hot reconciliation queries:
--   * "does this script already have a managed schedule?"   (script_path lookup)
--   * "drop any managed schedules for this deleted script"  (same lookup)
-- The boolean predicate keeps the index narrow (only managed rows are stored).
CREATE INDEX IF NOT EXISTS idx_schedule_managed
    ON schedule (workspace_id, script_path)
    WHERE managed;
