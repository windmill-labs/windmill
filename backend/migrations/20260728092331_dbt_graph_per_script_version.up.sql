-- The dbt graph belongs to a script VERSION, not to a path.
--
-- Keyed by path alone, a deploy overwrote the only copy, so a run page could
-- only ever render today's project: an older run showed today's models, SQL and
-- `ref()` lineage no matter what it had actually run. Keying by the hash the job
-- recorded lets each deployed version keep its own graph, and every job already
-- carries `v2_job.runnable_id` naming the version it ran.
--
-- Per DEPLOY, not per run: ten thousand runs of one version share one graph.
ALTER TABLE dbt_node ADD COLUMN IF NOT EXISTS script_hash BIGINT;
ALTER TABLE dbt_edge ADD COLUMN IF NOT EXISTS script_hash BIGINT;

-- Existing rows describe whatever was deployed last, so they belong to the
-- newest live version of their path. A row whose script is gone describes a
-- project that no longer exists and is dropped rather than given a version.
UPDATE dbt_node n SET script_hash = (
    SELECT s.hash FROM script s
     WHERE s.workspace_id = n.workspace_id AND s.path = n.script_path
       AND s.deleted = false AND s.archived = false
     ORDER BY s.created_at DESC LIMIT 1)
 WHERE script_hash IS NULL;
UPDATE dbt_edge e SET script_hash = (
    SELECT s.hash FROM script s
     WHERE s.workspace_id = e.workspace_id AND s.path = e.script_path
       AND s.deleted = false AND s.archived = false
     ORDER BY s.created_at DESC LIMIT 1)
 WHERE script_hash IS NULL;
DELETE FROM dbt_node WHERE script_hash IS NULL;
DELETE FROM dbt_edge WHERE script_hash IS NULL;

ALTER TABLE dbt_node ALTER COLUMN script_hash SET NOT NULL;
ALTER TABLE dbt_edge ALTER COLUMN script_hash SET NOT NULL;

-- The version joins the key: two deploys of one path now write disjoint rows,
-- which is what removes the advisory lock and the "am I still newest" claim the
-- single-copy model needed.
ALTER TABLE dbt_node DROP CONSTRAINT dbt_node_pkey;
ALTER TABLE dbt_node ADD PRIMARY KEY (workspace_id, script_path, script_hash, unique_id);
ALTER TABLE dbt_edge DROP CONSTRAINT dbt_edge_pkey;
ALTER TABLE dbt_edge
    ADD PRIMARY KEY (workspace_id, script_path, script_hash, parent_unique_id, child_unique_id);

-- Nothing pruned these before because there was one copy per path; now they
-- accumulate per deploy, so they die with the version that produced them.
-- Composite because `script`'s own key is (workspace_id, hash): a hash is unique
-- only within its workspace, so referencing it alone has nothing to point at.
ALTER TABLE dbt_node
    ADD CONSTRAINT dbt_node_script_fkey FOREIGN KEY (workspace_id, script_hash)
    REFERENCES script (workspace_id, hash) ON DELETE CASCADE;
ALTER TABLE dbt_edge
    ADD CONSTRAINT dbt_edge_script_fkey FOREIGN KEY (workspace_id, script_hash)
    REFERENCES script (workspace_id, hash) ON DELETE CASCADE;
