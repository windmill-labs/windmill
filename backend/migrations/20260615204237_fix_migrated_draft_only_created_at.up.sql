-- Repair `created_at` for the draft-only items migrated by
-- 20260609165313_remove_draft_only. That migration inserted the legacy
-- (email IS NULL) draft stubs without an explicit `created_at`, so every row
-- it created defaulted to the migration's `now()` — which is
-- `transaction_timestamp()`, constant for the whole transaction. They all
-- landed at the migration instant and, sorted newest-first, flooded the top
-- of the home list.
--
-- The original per-item timestamps are unrecoverable (the source script/flow/
-- app rows were deleted by that migration and the draft value carries no
-- timestamp), so reset them to the epoch: these never-deployed, never-resaved
-- stubs sort to the bottom instead of the top. Editing one later bumps its
-- `created_at` to now() and it floats back up naturally.
--
-- Identification is exact and safe. sqlx applies a transactional migration and
-- inserts its `_sqlx_migrations` bookkeeping row in ONE transaction, both via
-- `DEFAULT now()`, so that row's `installed_on` is byte-identical to the
-- `created_at` of every row the migration inserted. Matching on it touches
-- only those rows and skips any edited since (their `created_at` was bumped,
-- so it no longer equals `installed_on`). If the values somehow don't match,
-- this updates nothing — it can never clobber a real draft.
--
-- Both columns are TIMESTAMPTZ (draft.created_at since
-- 20260514233244_convert_draft_created_at_to_timestamptz), so this is an exact
-- instant comparison — no implicit timestamp/timestamptz cast or timezone
-- sensitivity.
UPDATE draft d
SET created_at = 'epoch'
FROM _sqlx_migrations m
WHERE m.version = 20260609165313
  AND d.email IS NULL
  AND d.typ IN ('script', 'flow', 'app', 'raw_app')
  AND d.created_at = m.installed_on;
