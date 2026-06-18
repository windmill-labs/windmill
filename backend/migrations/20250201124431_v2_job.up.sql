-- Add up migration script here
CREATE TYPE job_trigger_kind AS ENUM (
    'webhook', 'http', 'websocket', 'kafka', 'email', 'nats', 'schedule', 'app', 'ui', 'postgres'
);

ALTER TABLE v2_job
    ADD COLUMN IF NOT EXISTS created_at TIMESTAMP WITH TIME ZONE DEFAULT now() NOT NULL,
    ADD COLUMN IF NOT EXISTS created_by VARCHAR(255) DEFAULT 'missing' NOT NULL,
    ADD COLUMN IF NOT EXISTS permissioned_as VARCHAR(55) DEFAULT 'g/all' NOT NULL,
    ADD COLUMN IF NOT EXISTS permissioned_as_email VARCHAR(255) DEFAULT 'missing@email.xyz' NOT NULL,
    ADD COLUMN IF NOT EXISTS kind job_kind DEFAULT 'script'::job_kind NOT NULL,
    ADD COLUMN IF NOT EXISTS runnable_id BIGINT,
    ADD COLUMN IF NOT EXISTS runnable_path VARCHAR(255),
    ADD COLUMN IF NOT EXISTS parent_job UUID,
    ADD COLUMN IF NOT EXISTS root_job UUID,
    ADD COLUMN IF NOT EXISTS script_lang script_lang DEFAULT 'python3'::script_lang,
    ADD COLUMN IF NOT EXISTS script_entrypoint_override VARCHAR(255),
    ADD COLUMN IF NOT EXISTS flow_step INTEGER,
    ADD COLUMN IF NOT EXISTS flow_step_id VARCHAR(255),
    ADD COLUMN IF NOT EXISTS flow_innermost_root_job UUID,
    ADD COLUMN IF NOT EXISTS trigger VARCHAR(255),
    ADD COLUMN IF NOT EXISTS trigger_kind job_trigger_kind,
    ADD COLUMN IF NOT EXISTS same_worker BOOLEAN DEFAULT FALSE NOT NULL,
    ADD COLUMN IF NOT EXISTS visible_to_owner BOOLEAN DEFAULT TRUE NOT NULL,
    ADD COLUMN IF NOT EXISTS concurrent_limit INTEGER,
    ADD COLUMN IF NOT EXISTS concurrency_time_window_s INTEGER,
    ADD COLUMN IF NOT EXISTS cache_ttl INTEGER,
    ADD COLUMN IF NOT EXISTS timeout INTEGER,
    ADD COLUMN IF NOT EXISTS priority SMALLINT,
    ADD COLUMN IF NOT EXISTS preprocessed BOOLEAN,
    ADD COLUMN IF NOT EXISTS args JSONB,
    ADD COLUMN IF NOT EXISTS labels TEXT[],
    ADD COLUMN IF NOT EXISTS pre_run_error TEXT;

CREATE POLICY see_folder_extra_perms_user ON v2_job
    AS PERMISSIVE
    FOR ALL
    TO windmill_user
    USING ((visible_to_owner IS TRUE) AND (SPLIT_PART((runnable_path)::TEXT, '/'::TEXT, 1) = 'f'::TEXT) AND
           (SPLIT_PART((runnable_path)::TEXT, '/'::TEXT, 2) = ANY (
               REGEXP_SPLIT_TO_ARRAY(CURRENT_SETTING('session.folders_read'::TEXT), ','::TEXT))));

CREATE POLICY see_own_path ON v2_job
    AS PERMISSIVE
    FOR ALL
    TO windmill_user
    USING ((visible_to_owner IS TRUE) AND (SPLIT_PART((runnable_path)::TEXT, '/'::TEXT, 1) = 'u'::TEXT) AND
           (SPLIT_PART((runnable_path)::TEXT, '/'::TEXT, 2) = CURRENT_SETTING('session.user'::TEXT)));

CREATE POLICY see_member_path ON v2_job
    AS PERMISSIVE
    FOR ALL
    TO windmill_user
    USING ((visible_to_owner IS TRUE) AND (SPLIT_PART((runnable_path)::TEXT, '/'::TEXT, 1) = 'g'::TEXT) AND
           (SPLIT_PART((runnable_path)::TEXT, '/'::TEXT, 2) = ANY
            (REGEXP_SPLIT_TO_ARRAY(CURRENT_SETTING('session.groups'::TEXT), ','::TEXT))));

CREATE POLICY see_own ON v2_job
    AS PERMISSIVE
    FOR ALL
    TO windmill_user
    USING ((SPLIT_PART((permissioned_as)::TEXT, '/'::TEXT, 1) = 'f'::TEXT) AND
           (SPLIT_PART((permissioned_as)::TEXT, '/'::TEXT, 2) = CURRENT_SETTING('session.user'::TEXT)));

CREATE POLICY see_member ON v2_job
    AS PERMISSIVE
    FOR ALL
    TO windmill_user
    USING ((SPLIT_PART((permissioned_as)::TEXT, '/'::TEXT, 1) = 'g'::TEXT) AND
           (SPLIT_PART((permissioned_as)::TEXT, '/'::TEXT, 2) = ANY
            (REGEXP_SPLIT_TO_ARRAY(CURRENT_SETTING('session.groups'::TEXT), ','::TEXT))));

CREATE POLICY admin_policy ON v2_job
    AS PERMISSIVE
    FOR ALL
    TO windmill_admin;

GRANT ALL ON v2_job TO windmill_user, windmill_admin;
