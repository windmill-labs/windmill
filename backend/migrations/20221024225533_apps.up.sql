-- Add up migration script here
CREATE TYPE EXECUTION_MODE AS ENUM ('anonymous', 'publisher', 'viewer');

CREATE TABLE app (
    id SERIAL PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
	path varchar(255) NOT NULL,
    summary VARCHAR(1000) NOT NULL DEFAULT '',
    policy JSONB NOT NULL,
    versions BIGINT[] NOT NULL,
    execution_mode EXECUTION_MODE NOT NULL DEFAULT 'publisher',
    extra_perms JSONB NOT NULL DEFAULT '{}'
);

CREATE TABLE app_version(
    id BIGSERIAL PRIMARY KEY,
    flow_id BIGINT NOT NULL,
    value JSONB NOT NULL,
	created_by VARCHAR(50) NOT NULL,
	created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    FOREIGN KEY (flow_id) REFERENCES app(id) ON DELETE CASCADE
);