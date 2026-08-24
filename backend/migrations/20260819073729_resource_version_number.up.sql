-- `id` is one identity sequence for the whole table and stays how a version is addressed;
-- `version` is the resource's own count, which is what a version is presented by.
ALTER TABLE resource_version ADD COLUMN version BIGINT;

UPDATE resource_version rv SET version = ranked.rn
FROM (
    SELECT id, row_number() OVER (PARTITION BY workspace_id, path ORDER BY id) AS rn
    FROM resource_version
) ranked
WHERE rv.id = ranked.id;

ALTER TABLE resource_version ALTER COLUMN version SET NOT NULL;

-- The number is only meaningful within a path, so the triple is the natural key: it serves the
-- lookup by number and makes a duplicate a hard error rather than two rows claiming v7.
CREATE UNIQUE INDEX index_resource_version_number ON resource_version (workspace_id, path, version);

-- Numbering is assigned here rather than derived when read because both ways of deleting versions
-- take the oldest ones: the monitor's trim past MAX_RESOURCE_VERSIONS, and clearing a history down
-- to its current value. A number computed by counting the survivors would renumber under either,
-- so a run recorded against v3 would later name a different version.
CREATE OR REPLACE FUNCTION record_resource_version() RETURNS trigger AS $$
BEGIN
    -- `session.user` is set by UserDB::begin for authed requests; worker and system writes fall
    -- back to the row's own author. NULLIF because a transaction-local set_config resets the
    -- placeholder to the empty string rather than unsetting it, so a pooled connection that
    -- previously served an authed request reports '' here, not NULL.
    --
    -- MAX + 1 needs no lock of its own: this runs inside the transaction that wrote `resource`, and
    -- a concurrent write to the same path blocks on that row's lock — or on the primary key, for an
    -- insert — before its own trigger can run, so the maximum cannot be read stale. Deleting
    -- versions never lowers it, since both deletions keep the newest row.
    INSERT INTO resource_version (workspace_id, path, resource_type, value, created_by, version)
    VALUES (
        NEW.workspace_id, NEW.path, NEW.resource_type, NEW.value,
        COALESCE(NULLIF(current_setting('session.user', true), ''), NEW.created_by),
        (SELECT COALESCE(MAX(version), 0) + 1 FROM resource_version
         WHERE workspace_id = NEW.workspace_id AND path = NEW.path)
    );

    -- The per-path cap is enforced by trim_resource_versions in the monitor, not here: trimming
    -- on every write would tax a path `setResource` can drive in a loop, to keep a bound that
    -- does not need to hold instantaneously.

    RETURN NEW;
END;
-- SECURITY DEFINER so history is written on behalf of every writer without granting anyone direct
-- write access to the table, which users hold SELECT on only. `SET search_path FROM CURRENT` is the
-- injection hardening that goes with it, captured rather than hardcoded so installs running a
-- non-public PG_SCHEMA still resolve (see
-- 20260624103600_repair_folder_labels_search_path.up.sql).
$$ LANGUAGE plpgsql SECURITY DEFINER SET search_path FROM CURRENT;
