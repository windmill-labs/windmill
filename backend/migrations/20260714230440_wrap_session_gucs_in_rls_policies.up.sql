-- Wrap per-row session GUC reads in RLS predicates in a scalar sub-select so
-- Postgres hoists them to a one-time InitPlan instead of re-evaluating
-- current_setting('session.*') once per scanned row.
--
-- The session GUCs are set with SET LOCAL (set_config(..., true)) in
-- set_session_context(), so they are constant for the duration of a statement.
-- Wrapping the session-derived subexpression in (select ...) is therefore
-- value-preserving: same rows in, same rows out, N per-row GUC lookups collapse
-- to 1. Array-producing subexpressions keep an explicit ::text[] cast on the
-- sub-select so `= ANY (...)` / `?|` stay in their array-operand form rather
-- than being reparsed as a row-returning subquery.
--
-- This rewrites every existing policy (recorded across ~30 prior migrations)
-- whose USING / WITH CHECK predicate reads a session GUC, by deparsing the
-- current predicate and substituting the wrapped forms. See the .down.sql for
-- the inverse.

CREATE FUNCTION pg_temp.wrap_session_gucs(expr text) RETURNS text AS $fn$
DECLARE e text := expr;
BEGIN
  IF e IS NULL THEN
    RETURN NULL;
  END IF;
  -- Protect the maximal session-derived subexpressions with placeholders first,
  -- so the bare-scalar pass below only touches genuinely-bare session.user reads.
  e := replace(e, 'regexp_split_to_array(current_setting(''session.pgroups''::text), '',''::text)', '@@WM_P0@@');
  e := replace(e, 'regexp_split_to_array(current_setting(''session.groups''::text), '',''::text)', '@@WM_P1@@');
  e := replace(e, 'regexp_split_to_array(current_setting(''session.folders_read''::text), '',''::text)', '@@WM_P2@@');
  e := replace(e, 'regexp_split_to_array(current_setting(''session.folders_write''::text), '',''::text)', '@@WM_P3@@');
  e := replace(e, 'concat(''u/'', current_setting(''session.user''::text))', '@@WM_P4@@');
  -- Wrap any remaining bare scalar session.user read.
  e := replace(e, 'current_setting(''session.user''::text)', '(select current_setting(''session.user''::text))');
  -- Restore the protected subexpressions, now wrapped in a scalar sub-select.
  e := replace(e, '@@WM_P0@@', '(select regexp_split_to_array(current_setting(''session.pgroups''::text), '',''::text))::text[]');
  e := replace(e, '@@WM_P1@@', '(select regexp_split_to_array(current_setting(''session.groups''::text), '',''::text))::text[]');
  e := replace(e, '@@WM_P2@@', '(select regexp_split_to_array(current_setting(''session.folders_read''::text), '',''::text))::text[]');
  e := replace(e, '@@WM_P3@@', '(select regexp_split_to_array(current_setting(''session.folders_write''::text), '',''::text))::text[]');
  e := replace(e, '@@WM_P4@@', '(select concat(''u/'', current_setting(''session.user''::text)))');
  RETURN e;
END;
$fn$ LANGUAGE plpgsql IMMUTABLE;

DO $do$
DECLARE
  r record;
  new_qual text;
  new_check text;
  stmt text;
BEGIN
  FOR r IN
    SELECT schemaname, tablename, policyname, permissive, cmd, roles, qual, with_check
    FROM pg_policies
    WHERE schemaname = 'public'
      AND (coalesce(qual, '') LIKE '%current_setting(''session.%'
           OR coalesce(with_check, '') LIKE '%current_setting(''session.%')
  LOOP
    new_qual := pg_temp.wrap_session_gucs(r.qual);
    new_check := pg_temp.wrap_session_gucs(r.with_check);

    EXECUTE format('DROP POLICY %I ON %I.%I', r.policyname, r.schemaname, r.tablename);

    stmt := format(
      'CREATE POLICY %I ON %I.%I AS %s FOR %s TO %s',
      r.policyname, r.schemaname, r.tablename,
      r.permissive, r.cmd,
      (SELECT string_agg(quote_ident(role_name), ', ') FROM unnest(r.roles) AS role_name)
    );
    IF new_qual IS NOT NULL THEN
      stmt := stmt || format(' USING (%s)', new_qual);
    END IF;
    IF new_check IS NOT NULL THEN
      stmt := stmt || format(' WITH CHECK (%s)', new_check);
    END IF;

    EXECUTE stmt;
  END LOOP;
END;
$do$;

DROP FUNCTION pg_temp.wrap_session_gucs(text);
