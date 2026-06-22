-- Per-user lifetime usage of the Windmill-provided free Claude Opus tier.
-- Keyed by normalized email so the allowance is shared across all of a user's
-- workspaces (and resistant to +tag / gmail-dot aliasing).
CREATE TABLE ai_free_token_usage (
    email VARCHAR(255) PRIMARY KEY,
    tokens_used BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Instance-wide daily ceiling for the free tier (a cost kill-switch independent of
-- per-user budgets). One row per UTC day.
CREATE TABLE ai_free_token_daily_usage (
    day DATE PRIMARY KEY,
    tokens_used BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
