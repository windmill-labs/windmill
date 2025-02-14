-- Add up migration script here
DROP POLICY IF EXISTS see_own ON v2_job;
CREATE POLICY see_own ON v2_job
    AS PERMISSIVE
    FOR ALL
    TO windmill_user
    USING ((SPLIT_PART((permissioned_as)::TEXT, '/'::TEXT, 1) = 'u'::TEXT) AND
           (SPLIT_PART((permissioned_as)::TEXT, '/'::TEXT, 2) = CURRENT_SETTING('session.user'::TEXT)));