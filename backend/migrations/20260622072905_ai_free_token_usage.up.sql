-- Per-user lifetime usage of the Windmill-provided free Claude Opus tier, measured
-- as cost in nano-dollars (1e-9 USD) of Opus 4.8-equivalent spend rather than raw
-- tokens — cache reads cost 0.1x a normal input token, so a token count wildly
-- overstates the real bill. Keyed by normalized email so the allowance is shared
-- across a user's workspaces (and resistant to +tag / gmail-dot aliasing).
CREATE TABLE ai_free_token_usage (
    email VARCHAR(255) PRIMARY KEY,
    cost_nanos BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Instance-wide daily cost ceiling (nano-dollars) for the free tier — a kill-switch
-- independent of per-user budgets. One row per UTC day.
CREATE TABLE ai_free_token_daily_usage (
    day DATE PRIMARY KEY,
    cost_nanos BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
