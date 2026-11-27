-- The risks a token review flags, computed once for the user's token list and the superadmin
-- review (windmill-api-users/src/users.rs). `self_managed` marks a token the server mints and
-- expires or renews on its own, which nobody manages by hand; its `risks` are empty. The 90 days
-- are mirrored by STALE_TOKEN_DAYS in frontend/src/lib/components/settings/tokenRisks.ts,
-- which only words them.
CREATE OR REPLACE VIEW token_risk AS
SELECT
    t.email,
    t.label,
    t.token_prefix,
    t.expiration,
    t.created_at,
    t.last_used_at,
    t.scopes,
    t.workspace_id,
    t.read_only,
    t.allowed_ips,
    m.self_managed,
    CASE WHEN m.self_managed THEN ARRAY[]::text[] ELSE array_remove(ARRAY[
        -- Scopes or read-only mode narrow a token's rights; a job tag filter alone does not
        -- (windmill-api-auth/src/scopes.rs).
        CASE WHEN m.unrestricted AND t.super_admin THEN 'superadmin' END,
        CASE WHEN m.unrestricted AND NOT t.super_admin AND t.admin
                  AND a.admin_of IS NOT NULL
                  AND (t.workspace_id IS NULL OR t.workspace_id = ANY(a.admin_of))
             THEN 'admin' END,
        CASE WHEN t.workspace_id IS NULL THEN 'all_workspaces' END,
        CASE WHEN t.expiration IS NULL THEN 'never_expires' END,
        CASE WHEN t.last_used_at < now() - interval '90 days' THEN 'unused' END
    ], NULL) END AS risks
FROM token t
LEFT JOIN (
    SELECT email, array_agg(workspace_id) AS admin_of FROM usr WHERE is_admin GROUP BY email
) a ON a.email = t.email
CROSS JOIN LATERAL (
    SELECT
        -- A job token, or a label only the server can mint (`is_server_minted_label`).
        (t.job IS NOT NULL
            OR t.label IN ('session', 'guest_session', 'ephemeral-script')
            OR starts_with(t.label, 'ephemeral-webhook-')
            OR starts_with(t.label, 'ephemeral-script-end-user-')
            OR starts_with(t.label, 'mcp-oauth-')
            -- `tokens/create` accepts these labels too, so only a lifetime as short as the
            -- system's own (72h for the editor's language server at most) proves the token is
            -- one of those rather than a long-lived token borrowing the label.
            OR ((t.label = 'debugger-token'
                    OR lower(t.label) LIKE 'ephemeral%'
                    OR starts_with(t.label, 'embed_app:')
                    OR starts_with(t.label, 'sdk_app:')
                    OR starts_with(t.label, 'impersonation:'))
                AND t.expiration <= t.created_at + interval '3 days')) IS TRUE AS self_managed,
        NOT t.read_only
            AND NOT EXISTS (SELECT 1 FROM unnest(t.scopes) s WHERE NOT starts_with(s, 'if_jobs:filter_tags:'))
            AS unrestricted
) m;
