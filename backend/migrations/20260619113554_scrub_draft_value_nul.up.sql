-- One-time cleanup of drafts whose `json` value carries a real U+0000 (NUL)
-- escape — storable only because `draft.value` is `json`, not `jsonb`. Such a
-- value makes any `->>`/`to_jsonb` extraction raise `22P05`, which 500'd
-- GET /drafts/list. New writes are sanitized in the application layer
-- (update_draft → strip_json_nul); this fixes rows written before that landed.
--
-- Only genuinely-poisoned rows are touched: a real NUL makes `value::jsonb`
-- raise, which distinguishes it from a legitimately escaped backslash sequence
-- (which `jsonb` accepts). The text replace handles the real-world shape — a NUL
-- inside a text field. A contrived value where stripping the escape leaves
-- invalid JSON is left as-is (and can no longer be created).
DO $$
DECLARE r RECORD;
BEGIN
    FOR r IN
        SELECT id, value FROM draft WHERE position(E'\\u0000' in value::text) > 0
    LOOP
        BEGIN
            PERFORM r.value::jsonb; -- not poisoned (legit escaped backslash): skip
        EXCEPTION WHEN others THEN
            BEGIN
                UPDATE draft
                SET value = replace(r.value::text, E'\\u0000', '')::json
                WHERE id = r.id;
            EXCEPTION WHEN others THEN
                NULL; -- pathological shape; cannot strip in SQL, no longer creatable
            END;
        END;
    END LOOP;
END $$;
