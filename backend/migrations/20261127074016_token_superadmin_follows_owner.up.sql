-- A token's `super_admin` column is what grants it superadmin power at auth time, so it must not
-- outlive the owner's: whatever path demotes, renames or deletes a superadmin `password` row,
-- that address's tokens lose the flag. Deferred to commit and judged on the committed state, so a
-- path that deletes and re-inserts the rows (the global user import) demotes only the addresses
-- that come back without superadmin, and a rename finds no tokens left under the old address.
-- Promotion is left to the application, which deliberately does not elevate MCP tokens.
-- Tokens with no `password` row at all (dedicated worker and runner group subprocesses) are
-- never matched, since the trigger fires only for a row that was a superadmin.
CREATE OR REPLACE FUNCTION demote_tokens_of_demoted_owner()
RETURNS TRIGGER AS $$
BEGIN
    WITH demoted AS (
        UPDATE token SET super_admin = false
         WHERE email = OLD.email
           AND super_admin
           AND NOT EXISTS (SELECT 1 FROM password WHERE email = OLD.email AND super_admin)
        RETURNING token_prefix
    )
    INSERT INTO notify_event (channel, payload)
    SELECT 'notify_token_invalidation', token_prefix FROM demoted;
    RETURN NULL;
END;
$$ LANGUAGE plpgsql SECURITY DEFINER;

DO
$do$
BEGIN
    EXECUTE format(
        'ALTER FUNCTION demote_tokens_of_demoted_owner() SET search_path = %I, pg_temp',
        current_schema()
    );
END
$do$;
REVOKE EXECUTE ON FUNCTION demote_tokens_of_demoted_owner() FROM PUBLIC;

DROP TRIGGER IF EXISTS password_superadmin_demote_tokens ON password;
CREATE CONSTRAINT TRIGGER password_superadmin_demote_tokens
AFTER UPDATE OF super_admin, email OR DELETE ON password
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW
WHEN (OLD.super_admin)
EXECUTE FUNCTION demote_tokens_of_demoted_owner();

-- Tokens already left with the flag by an owner who lost it, or whose account is gone. The
-- subprocess tokens a dedicated worker or runner group mints for itself are superadmin by
-- design and have no account.
UPDATE token t SET super_admin = false
 WHERE t.super_admin
   AND NOT EXISTS (SELECT 1 FROM password p WHERE p.email = t.email AND p.super_admin)
   AND NOT (t.label IN ('dedicated_worker', 'runner_group')
            AND t.email = t.label || '@windmill.dev');
