-- A token's `super_admin` column is what grants it superadmin power at auth time, so it must not
-- outlive the owner's: whatever path demotes or deletes a superadmin `password` row, that
-- address's tokens lose the flag. Deferred to commit and judged on the committed state, so a path
-- that deletes and re-inserts the rows (the global user import) demotes only the addresses that
-- come back without superadmin. An email change is not a demotion and does not fire: SCIM's
-- rename moves the account but leaves its tokens under the old address.
-- Promotion is left to the application, which deliberately does not elevate MCP tokens.
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
AFTER UPDATE OF super_admin OR DELETE ON password
DEFERRABLE INITIALLY DEFERRED
FOR EACH ROW
WHEN (OLD.super_admin)
EXECUTE FUNCTION demote_tokens_of_demoted_owner();

-- Tokens already left with the flag by an owner who lost it. A token whose address has no
-- `password` row is left alone: a SCIM rename and the subprocesses of dedicated workers and
-- runner groups both produce one, and cannot be told apart from a deleted account's.
UPDATE token t SET super_admin = false
  FROM password p
 WHERE p.email = t.email
   AND t.super_admin
   AND NOT p.super_admin;
