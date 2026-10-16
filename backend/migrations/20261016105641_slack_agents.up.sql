-- The Slack channels an agent is shared in: anyone in one of them can ask it, and it runs as its
-- identity (`on_behalf_of`). Keyed by the Slack team rather than the workspace, as channels are,
-- and at most one agent per channel answers a message that names none. The agent key cascades,
-- so a renamed agent keeps its channels and a deleted one leaves them.
CREATE TABLE slack_agent_channel (
    slack_team_id VARCHAR(255) NOT NULL,
    channel_id VARCHAR(255) NOT NULL,
    channel_name VARCHAR(255) NOT NULL,
    workspace_id VARCHAR(50) NOT NULL,
    agent_path VARCHAR(255) NOT NULL,
    is_default BOOLEAN NOT NULL DEFAULT false,
    PRIMARY KEY (slack_team_id, channel_id, workspace_id, agent_path),
    FOREIGN KEY (workspace_id, agent_path) REFERENCES resource (workspace_id, path)
        ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX index_slack_agent_channel_workspace ON slack_agent_channel (workspace_id, agent_path);
CREATE UNIQUE INDEX index_slack_agent_channel_default ON slack_agent_channel (slack_team_id, channel_id)
    WHERE is_default;

-- The agent a Slack thread was started with, so a follow-up that names no agent goes to it.
CREATE TABLE slack_thread_agent (
    slack_team_id VARCHAR(255) NOT NULL,
    channel_id VARCHAR(255) NOT NULL,
    thread_ts VARCHAR(255) NOT NULL,
    workspace_id VARCHAR(50) NOT NULL,
    agent_path VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (slack_team_id, channel_id, thread_ts),
    FOREIGN KEY (workspace_id, agent_path) REFERENCES resource (workspace_id, path)
        ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX index_slack_thread_agent_workspace ON slack_thread_agent (workspace_id, agent_path);
CREATE INDEX index_slack_thread_agent_created_at ON slack_thread_agent (created_at);

-- The Slack messages an agent has taken up. Slack resends an event it did not see acknowledged,
-- and posts a direct message mentioning the bot as two events; only the first is answered.
CREATE TABLE slack_answered_message (
    slack_team_id VARCHAR(255) NOT NULL,
    channel_id VARCHAR(255) NOT NULL,
    ts VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (slack_team_id, channel_id, ts)
);

CREATE INDEX index_slack_answered_message_created_at ON slack_answered_message (created_at);

GRANT ALL ON slack_answered_message TO windmill_admin;
GRANT ALL ON slack_answered_message TO windmill_user;
GRANT ALL ON slack_agent_channel TO windmill_admin;
GRANT ALL ON slack_agent_channel TO windmill_user;
GRANT ALL ON slack_thread_agent TO windmill_admin;
GRANT ALL ON slack_thread_agent TO windmill_user;
