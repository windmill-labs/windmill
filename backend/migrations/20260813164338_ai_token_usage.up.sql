-- Per-workspace AI token spend, accumulated from the chat client. Rows hold token
-- counts rather than money: prices live in the frontend price table plus the
-- workspace's `ai_config.model_pricing` overrides and are applied at read time, so
-- correcting a price also corrects the history. `reported_cost_nano_usd` is the
-- exception — a few providers (OpenRouter) return what they actually charged, and
-- that figure wins over the estimate.
--
-- Distinct from `feature_usage`, which is anonymous telemetry that leaves the
-- instance and is pruned after 60 days; spend is per-user and kept.
CREATE TABLE ai_token_usage (
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id) ON DELETE CASCADE,
    day DATE NOT NULL DEFAULT CURRENT_DATE,
    email VARCHAR(255) NOT NULL,
    provider VARCHAR(50) NOT NULL,
    model VARCHAR(255) NOT NULL,
    -- Empty for chats that are not attached to an AI session.
    session_id VARCHAR(50) NOT NULL DEFAULT '',
    -- Uncached input only; the two cache columns hold the rest of the prompt, so
    -- each column maps to exactly one price and they never double-count.
    input_tokens BIGINT NOT NULL DEFAULT 0,
    cache_read_tokens BIGINT NOT NULL DEFAULT 0,
    cache_write_tokens BIGINT NOT NULL DEFAULT 0,
    output_tokens BIGINT NOT NULL DEFAULT 0,
    reported_cost_nano_usd BIGINT,
    requests BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (workspace_id, day, email, provider, model, session_id)
);

-- The usage listing filters on workspace and a date range; the PK only reaches
-- `day` through `email`, so it cannot serve that on its own.
CREATE INDEX idx_ai_token_usage_ws_day ON ai_token_usage (workspace_id, day DESC);

GRANT ALL ON ai_token_usage TO windmill_admin;
GRANT ALL ON ai_token_usage TO windmill_user;

-- Both handlers go through the raw pool, so no policy is needed for them to work.
-- Enabling RLS with an admin-only policy is the backstop: a future query that
-- reaches this table through UserDB sees nothing rather than every user's spend.
ALTER TABLE ai_token_usage ENABLE ROW LEVEL SECURITY;

CREATE POLICY admin_policy ON ai_token_usage FOR ALL TO windmill_admin USING (true);
