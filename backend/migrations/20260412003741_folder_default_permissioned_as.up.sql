-- Array of { path_glob, permissioned_as } rules. Globs are evaluated relative
-- to the folder root and matched in array order (first match wins). Applied at
-- create-time only, for admins and wm_deployers members.
ALTER TABLE folder
    ADD COLUMN default_permissioned_as JSONB NOT NULL DEFAULT '[]'::jsonb;
