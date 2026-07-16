-- One-time grant of the Windmill-provided free AI tier, measured as cost in nano-dollars
-- (1e-9 USD) rather than raw tokens — a prompt-cache hit costs a fraction of a fresh input
-- token, so a token count wildly overstates the real bill. The grant never resets: once
-- spent, the user must bring their own API key. Keyed by normalized email so the allowance
-- is shared across a user's workspaces (and is resistant to +tag / gmail-dot aliasing).
CREATE TABLE ai_free_token_usage (
    email VARCHAR(255) PRIMARY KEY,
    cost_nanos BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Instance-wide daily cost ceiling (nano-dollars) for the free tier — a kill-switch
-- independent of the per-user grant, bounding the blast radius of a bad day. One row per
-- UTC day.
CREATE TABLE ai_free_token_daily_usage (
    day DATE PRIMARY KEY,
    cost_nanos BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
