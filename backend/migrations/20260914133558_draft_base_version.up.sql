-- The version a draft forked from, as one opaque text id whatever the kind: a
-- script hash (hex), a flow_version.id, an app_version.id. NULL for a draft that
-- was never forked from a deploy and for kinds that keep no lineage.
ALTER TABLE draft ADD COLUMN base TEXT;

-- A U+0000 inside a `json` value makes `->>` raise 22P05; such rows keep NULL and
-- get their base on their next save.
UPDATE draft SET base = CASE typ::text
    WHEN 'script' THEN value ->> 'parent_hash'
    WHEN 'flow' THEN value ->> 'version_id'
    ELSE value ->> 'parent_version'
  END
WHERE typ::text IN ('script', 'flow', 'app', 'raw_app')
  AND position(chr(92) || 'u0000' in replace(value::text, chr(92) || chr(92), '')) = 0;
