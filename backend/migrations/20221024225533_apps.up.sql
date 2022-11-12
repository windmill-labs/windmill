-- Add up migration script here
CREATE TABLE app (
    id BIGSERIAL PRIMARY KEY,
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
	path varchar(255) NOT NULL,
    summary VARCHAR(1000) NOT NULL DEFAULT '',
    policy JSONB NOT NULL,
    versions BIGINT[] NOT NULL,
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

CREATE POLICY see_own ON app FOR ALL
USING (SPLIT_PART(app.path, '/', 1) = 'u' AND SPLIT_PART(app.path, '/', 2) = current_setting('session.user'));

CREATE POLICY see_member ON app FOR ALL
USING (SPLIT_PART(app.path, '/', 1) = 'g' AND SPLIT_PART(app.path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));
