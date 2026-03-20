-- Add permissioned_as column to all trigger tables and schedule
-- permissioned_as stores 'u/{username}', 'g/{group}', or raw email
-- We add nullable first, populate, then set NOT NULL to avoid a DEFAULT '' that could mask bugs.

-- Trigger tables: add permissioned_as, drop email
ALTER TABLE http_trigger ADD COLUMN permissioned_as VARCHAR(255);
UPDATE http_trigger SET permissioned_as = CASE WHEN edited_by LIKE '%@%' THEN edited_by ELSE 'u/' || edited_by END;
ALTER TABLE http_trigger ALTER COLUMN permissioned_as SET NOT NULL;
ALTER TABLE http_trigger DROP COLUMN email;

ALTER TABLE websocket_trigger ADD COLUMN permissioned_as VARCHAR(255);
UPDATE websocket_trigger SET permissioned_as = CASE WHEN edited_by LIKE '%@%' THEN edited_by ELSE 'u/' || edited_by END;
ALTER TABLE websocket_trigger ALTER COLUMN permissioned_as SET NOT NULL;
ALTER TABLE websocket_trigger DROP COLUMN email;

ALTER TABLE postgres_trigger ADD COLUMN permissioned_as VARCHAR(255);
UPDATE postgres_trigger SET permissioned_as = CASE WHEN edited_by LIKE '%@%' THEN edited_by ELSE 'u/' || edited_by END;
ALTER TABLE postgres_trigger ALTER COLUMN permissioned_as SET NOT NULL;
ALTER TABLE postgres_trigger DROP COLUMN email;

ALTER TABLE mqtt_trigger ADD COLUMN permissioned_as VARCHAR(255);
UPDATE mqtt_trigger SET permissioned_as = CASE WHEN edited_by LIKE '%@%' THEN edited_by ELSE 'u/' || edited_by END;
ALTER TABLE mqtt_trigger ALTER COLUMN permissioned_as SET NOT NULL;
ALTER TABLE mqtt_trigger DROP COLUMN email;

ALTER TABLE kafka_trigger ADD COLUMN permissioned_as VARCHAR(255);
UPDATE kafka_trigger SET permissioned_as = CASE WHEN edited_by LIKE '%@%' THEN edited_by ELSE 'u/' || edited_by END;
ALTER TABLE kafka_trigger ALTER COLUMN permissioned_as SET NOT NULL;
ALTER TABLE kafka_trigger DROP COLUMN email;

ALTER TABLE nats_trigger ADD COLUMN permissioned_as VARCHAR(255);
UPDATE nats_trigger SET permissioned_as = CASE WHEN edited_by LIKE '%@%' THEN edited_by ELSE 'u/' || edited_by END;
ALTER TABLE nats_trigger ALTER COLUMN permissioned_as SET NOT NULL;
ALTER TABLE nats_trigger DROP COLUMN email;

ALTER TABLE sqs_trigger ADD COLUMN permissioned_as VARCHAR(255);
UPDATE sqs_trigger SET permissioned_as = CASE WHEN edited_by LIKE '%@%' THEN edited_by ELSE 'u/' || edited_by END;
ALTER TABLE sqs_trigger ALTER COLUMN permissioned_as SET NOT NULL;
ALTER TABLE sqs_trigger DROP COLUMN email;

ALTER TABLE gcp_trigger ADD COLUMN permissioned_as VARCHAR(255);
UPDATE gcp_trigger SET permissioned_as = CASE WHEN edited_by LIKE '%@%' THEN edited_by ELSE 'u/' || edited_by END;
ALTER TABLE gcp_trigger ALTER COLUMN permissioned_as SET NOT NULL;
ALTER TABLE gcp_trigger DROP COLUMN email;

ALTER TABLE email_trigger ADD COLUMN permissioned_as VARCHAR(255);
UPDATE email_trigger SET permissioned_as = CASE WHEN edited_by LIKE '%@%' THEN edited_by ELSE 'u/' || edited_by END;
ALTER TABLE email_trigger ALTER COLUMN permissioned_as SET NOT NULL;
ALTER TABLE email_trigger DROP COLUMN email;

-- Schedule table: add permissioned_as, keep email for backwards compat with old workers
-- For superadmin-owned schedules, use the email to find the actual username (since edited_by
-- may have been overwritten by a later edit). Otherwise use edited_by as the source.
ALTER TABLE schedule ADD COLUMN permissioned_as VARCHAR(255);
UPDATE schedule SET permissioned_as = CASE
    WHEN EXISTS (
        SELECT 1 FROM password p WHERE p.email = schedule.email AND p.super_admin = true
    ) THEN COALESCE(
        'u/' || (SELECT u.username FROM usr u WHERE u.email = schedule.email AND u.workspace_id = schedule.workspace_id LIMIT 1),
        CASE WHEN edited_by LIKE '%@%' THEN edited_by ELSE 'u/' || edited_by END
    )
    ELSE CASE WHEN edited_by LIKE '%@%' THEN edited_by ELSE 'u/' || edited_by END
END;
ALTER TABLE schedule ALTER COLUMN permissioned_as SET NOT NULL;
