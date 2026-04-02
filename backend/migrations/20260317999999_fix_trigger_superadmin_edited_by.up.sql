-- Pre-fix: before permissioned_as migration drops the email column, update edited_by
-- for triggers where the user (edited_by) is not in the workspace but is a superadmin.
-- This ensures the subsequent 20260318000000 migration stores the raw email as permissioned_as
-- (via the `edited_by LIKE '%@%'` branch).
-- For instances that already applied 20260318000000, this is a no-op (email column is gone);
-- the 20260401000000 migration handles those as a fallback.

DO $$
DECLARE
    trigger_table TEXT;
    has_email BOOLEAN;
BEGIN
    FOREACH trigger_table IN ARRAY ARRAY[
        'http_trigger',
        'websocket_trigger',
        'postgres_trigger',
        'mqtt_trigger',
        'kafka_trigger',
        'nats_trigger',
        'sqs_trigger',
        'gcp_trigger',
        'email_trigger'
    ]
    LOOP
        SELECT EXISTS (
            SELECT 1 FROM information_schema.columns
            WHERE table_name = trigger_table AND column_name = 'email'
        ) INTO has_email;

        IF has_email THEN
            EXECUTE format($q$
                UPDATE %I t
                SET edited_by = t.email
                WHERE NOT EXISTS (
                    SELECT 1 FROM usr u
                    WHERE u.username = t.edited_by
                      AND u.workspace_id = t.workspace_id
                )
                AND EXISTS (
                    SELECT 1 FROM password p
                    WHERE p.email = t.email
                      AND p.super_admin = true
                )
            $q$, trigger_table);
        END IF;
    END LOOP;
END;
$$;
