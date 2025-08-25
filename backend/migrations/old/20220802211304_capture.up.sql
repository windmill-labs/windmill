CREATE TABLE capture (
    workspace_id  VARCHAR(50)   NOT NULL,
    path          VARCHAR(255)  NOT NULL,
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT now(),
    created_by    VARCHAR(50)   NOT NULL,
    payload       JSONB         NOT NULL DEFAULT 'null'::jsonb
                                CHECK (length(payload::text) < 10 * 1024),

    PRIMARY KEY (workspace_id, path),
    FOREIGN KEY (workspace_id)  REFERENCES workspace(id)
);

ALTER TABLE capture ENABLE ROW LEVEL SECURITY;

CREATE POLICY see_own ON capture FOR ALL
USING (    SPLIT_PART(capture.path, '/', 1) = 'u'
       AND SPLIT_PART(capture.path, '/', 2) = current_setting('session.user'));

CREATE POLICY see_member ON capture FOR ALL
USING (    SPLIT_PART(capture.path, '/', 1) = 'g'
       AND SPLIT_PART(capture.path, '/', 2) = any(regexp_split_to_array(current_setting('session.groups'), ',')::text[]));
