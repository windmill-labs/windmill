-- Create a new daily-partitioned audit table alongside the existing one.
-- New inserts go to audit_partitioned; reads UNION ALL both tables.
-- The old audit table empties out naturally via retention cleanup.

CREATE TABLE audit_partitioned (
    workspace_id VARCHAR(50) NOT NULL,
    id BIGINT NOT NULL DEFAULT nextval('audit_id_seq'),
    timestamp TIMESTAMPTZ NOT NULL DEFAULT now(),
    username VARCHAR(255) NOT NULL,
    operation VARCHAR(50) NOT NULL,
    action_kind ACTION_KIND NOT NULL,
    resource VARCHAR(255),
    parameters JSONB,
    email VARCHAR(255),
    span VARCHAR(255),
    PRIMARY KEY (id, timestamp)
) PARTITION BY RANGE (timestamp);

-- Create daily partitions for today + 3 days
DO $$
DECLARE
    curr_date DATE := CURRENT_DATE;
    end_date DATE := CURRENT_DATE + INTERVAL '3 days';
BEGIN
    WHILE curr_date <= end_date LOOP
        EXECUTE format(
            'CREATE TABLE %I PARTITION OF audit_partitioned FOR VALUES FROM (%L) TO (%L)',
            'audit_' || to_char(curr_date, 'YYYYMMDD'),
            curr_date,
            curr_date + INTERVAL '1 day'
        );
        curr_date := curr_date + INTERVAL '1 day';
    END LOOP;
END $$;

-- Indexes (auto-propagated to all current and future partitions)
CREATE INDEX ix_audit_partitioned_timestamps ON audit_partitioned (timestamp DESC);
CREATE INDEX idx_audit_partitioned_recent_login_activities
    ON audit_partitioned (timestamp, username)
    WHERE operation IN ('users.login', 'oauth.login', 'users.token.refresh');
