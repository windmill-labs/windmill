-- Add up migration script here
CREATE TABLE app (
    id SERIAL PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
	path varchar(255) NOT NULL,
    summary VARCHAR(1000) NOT NULL,
    versions BIGINT[] NOT NULL,
    publisher VARCHAR(50) NOT NULL,
    public BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE app_version(
    id BIGSERIAL PRIMARY KEY,
    value JSONB NOT NULL,
	created_by VARCHAR(50) NOT NULL,
	created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now()
);