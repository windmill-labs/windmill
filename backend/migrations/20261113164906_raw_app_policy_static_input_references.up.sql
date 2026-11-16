-- Store static input references in raw-app policies: where the latest version of a raw app gives a
-- static input as a `$var:`, `$jsonvar:` or `$res:` reference, its policy holds that reference.
DO $mig$
DECLARE
    r record;
BEGIN
    FOR r IN
        WITH latest AS MATERIALIZED (
            -- Read with its NUL escapes removed, as `strip_json_nul` does, since jsonb refuses them:
            -- backslash pairs are set aside first, so a `\u0000` after an escaped backslash stays.
            -- chr(1) cannot occur in json text, which keeps control characters escaped.
            SELECT a.id, a.policy,
                replace(replace(replace(v.value::text, chr(92) || chr(92), chr(1)), chr(92) || 'u0000', ''),
                    chr(1), chr(92) || chr(92))::jsonb AS value
            FROM app a
            JOIN app_version v ON v.id = a.versions[array_upper(a.versions, 1)]
            WHERE v.raw_app
              AND v.value::text ~ '"\$(var|jsonvar|res):'
        )
        SELECT l.id, t.key AS triggerable, f.key AS field, rf.value->'value' AS reference
        FROM latest l
        CROSS JOIN LATERAL jsonb_each(CASE jsonb_typeof(l.value->'runnables') WHEN 'object' THEN l.value->'runnables' END) rn
        CROSS JOIN LATERAL jsonb_each(CASE jsonb_typeof(rn.value->'fields') WHEN 'object' THEN rn.value->'fields' END) rf
        CROSS JOIN LATERAL jsonb_each(CASE jsonb_typeof(l.policy->'triggerables_v2') WHEN 'object' THEN l.policy->'triggerables_v2' END) t
        CROSS JOIN LATERAL jsonb_each(CASE jsonb_typeof(t.value->'static_inputs') WHEN 'object' THEN t.value->'static_inputs' END) f
        -- A key is `<runnable id>:<kind>/<hash or path>`. Ids may hold `:`, hashes and paths never
        -- do, so the id is exactly what precedes the last `:`.
        WHERE rn.key = left(t.key, length(t.key) - strpos(reverse(t.key), ':'))
          AND substr(t.key, length(rn.key) + 2) ~ '^(rawscript|script|flow|agent)/'
          AND f.key = rf.key
          AND rf.value->>'type' = 'static'
          AND rf.value->>'value' ~ '^\$(var|jsonvar|res):'
          AND f.value IS DISTINCT FROM rf.value->'value'
    LOOP
        UPDATE app
        SET policy = jsonb_set(policy, ARRAY['triggerables_v2', r.triggerable, 'static_inputs', r.field], r.reference)
        WHERE id = r.id;
    END LOOP;
-- Never blocks startup: an error undoes this block's changes and is logged instead.
EXCEPTION WHEN OTHERS THEN
    RAISE WARNING 'raw_app_policy_static_input_references: % (%), no policy changed', SQLERRM, SQLSTATE;
END
$mig$;
