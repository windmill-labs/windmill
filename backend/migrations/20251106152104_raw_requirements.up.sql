CREATE SEQUENCE IF NOT EXISTS workspace_dependencies_id_seq;

CREATE TABLE IF NOT EXISTS workspace_dependencies(
    id           BIGINT DEFAULT nextval('workspace_dependencies_id_seq') PRIMARY KEY,
    name         VARCHAR(255), -- If NULL - it's global
    content      TEXT NOT NULL,
    language     SCRIPT_LANG NOT NULL,
    description  text NOT NULL DEFAULT '',
    archived     BOOLEAN NOT NULL DEFAULT false,
    workspace_id character varying(50) NOT NULL,
    created_at   TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now()
);

-- Make any query that tries to create non-linear history fail
CREATE UNIQUE INDEX IF NOT EXISTS one_non_archived_per_name_language_constraint ON workspace_dependencies(name, language, workspace_id) WHERE archived = false AND name IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS one_non_archived_per_null_name_language_constraint ON workspace_dependencies(language, workspace_id) WHERE archived = false AND name IS NULL;

-- Performance indexes for common query patterns
-- For the list query (filtering by workspace_id and archived)
CREATE INDEX IF NOT EXISTS workspace_dependencies_workspace_archived_idx ON workspace_dependencies(workspace_id, archived) WHERE archived = false;

CREATE INDEX IF NOT EXISTS workspace_dependencies_workspace_lang_name_archived_idx ON workspace_dependencies(workspace_id, language, name, archived);
CREATE INDEX IF NOT EXISTS workspace_dependencies_id_workspace ON workspace_dependencies(id, workspace_id);
