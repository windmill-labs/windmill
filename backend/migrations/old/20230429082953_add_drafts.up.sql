-- Add up migration script here
CREATE TYPE DRAFT_TYPE AS ENUM ('script', 'flow', 'app');

CREATE TABLE draft (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    path VARCHAR(255) NOT NULL,
    typ DRAFT_TYPE NOT NULL,
    value JSONB NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, path, typ)
);

GRANT ALL ON draft TO windmill_admin;
GRANT ALL ON draft TO windmill_user;

ALTER TABLE script ADD COLUMN draft_only BOOLEAN;
ALTER TABLE flow ADD COLUMN draft_only BOOLEAN;
ALTER TABLE app ADD COLUMN draft_only BOOLEAN;
