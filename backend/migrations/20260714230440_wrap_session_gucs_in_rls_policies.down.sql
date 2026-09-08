-- Inverse of the up migration: strip the scalar sub-select wrapping from the
-- session GUC reads in RLS predicates, restoring the bare per-row forms.
--
-- Postgres re-deparses the wrapped sub-selects into a canonical shape
-- ( SELECT <expr> AS <alias>), and drops the redundant ::text[] cast in the
-- jsonb `?|` context while keeping it in the `= ANY (...)` context, so the
-- unwrap tolerates the alias and the optional trailing cast.

CREATE FUNCTION pg_temp.unwrap_session_gucs(expr text) RETURNS text AS $fn$
DECLARE e text := expr;
BEGIN
  IF e IS NULL THEN
    RETURN NULL;
  END IF;
  -- ( SELECT regexp_split_to_array(current_setting('session.<g>'::text), ','::text) AS regexp_split_to_array)[::text[]]
  e := regexp_replace(e,
    '\( SELECT (regexp_split_to_array\(current_setting\(''session\.(?:pgroups|groups|folders_read|folders_write)''::text\), '',''::text\)) AS regexp_split_to_array\)(?:::text\[\])?',
    '\1', 'g');
  -- ( SELECT concat('u/', current_setting('session.user'::text)) AS concat)
  e := regexp_replace(e,
    '\( SELECT (concat\(''u/'', current_setting\(''session\.user''::text\)\)) AS concat\)',
    '\1', 'g');
  -- ( SELECT current_setting('session.user'::text) AS current_setting)
  e := regexp_replace(e,
    '\( SELECT (current_setting\(''session\.user''::text\)) AS current_setting\)',
    '\1', 'g');
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
    new_qual := pg_temp.unwrap_session_gucs(r.qual);
    new_check := pg_temp.unwrap_session_gucs(r.with_check);

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

DROP FUNCTION pg_temp.unwrap_session_gucs(text);
