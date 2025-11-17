CREATE SEQUENCE IF NOT EXISTS workspace_dependencies_id_seq;

CREATE TABLE IF NOT EXISTS workspace_dependencies(
    id           BIGINT DEFAULT nextval('workspace_dependencies_id_seq') PRIMARY KEY,
    name         VARCHAR(255), -- If NULL - it's global
    content      TEXT NOT NULL,
    language     SCRIPT_LANG NOT NULL,
    archived     BOOLEAN NOT NULL DEFAULT false,
    workspace_id character varying(50) NOT NULL,
    created_at   TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now()
);

