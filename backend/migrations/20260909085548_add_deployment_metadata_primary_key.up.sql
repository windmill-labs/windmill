-- The sixth of the seven; why any of them need a key is in
-- 20260909052532_add_missing_primary_keys.
--
-- Kept out of that migration because it is the one table in the set with no retention
-- sweep -- rows accumulate per deployed script hash, flow version and app version for
-- the life of the instance -- so on an instance that upgrades after years of deploys
-- its ACCESS EXCLUSIVE rewrite is the one that could hold the others locked.
--
-- No natural key: each row is a script, flow OR app deployment, and the three unique
-- indexes are partial on exactly that split, so none of them covers every row.

ALTER TABLE deployment_metadata
    ADD COLUMN IF NOT EXISTS id BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY;
