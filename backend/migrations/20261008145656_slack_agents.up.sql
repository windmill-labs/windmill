-- The agent that answers in a Slack channel when a message names none. Keyed by the Slack team
-- rather than the workspace: one channel has one default, whichever workspace set it. The agent
-- key cascades, so a renamed agent keeps its channels and a deleted one releases them.
CREATE TABLE slack_channel_agent (
    slack_team_id VARCHAR(255) NOT NULL,
    channel_id VARCHAR(255) NOT NULL,
    channel_name VARCHAR(255) NOT NULL,
    workspace_id VARCHAR(50) NOT NULL,
    agent_path VARCHAR(255) NOT NULL,
    PRIMARY KEY (slack_team_id, channel_id),
    FOREIGN KEY (workspace_id, agent_path) REFERENCES resource (workspace_id, path)
        ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE INDEX index_slack_channel_agent_workspace ON slack_channel_agent (workspace_id, agent_path);

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
GRANT ALL ON slack_channel_agent TO windmill_admin;
GRANT ALL ON slack_channel_agent TO windmill_user;
GRANT ALL ON slack_thread_agent TO windmill_admin;
GRANT ALL ON slack_thread_agent TO windmill_user;
