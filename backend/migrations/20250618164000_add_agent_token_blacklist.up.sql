-- Add agent token blacklist table
CREATE TABLE agent_token_blacklist (
    token VARCHAR PRIMARY KEY,
    expires_at TIMESTAMP NOT NULL,
    blacklisted_at TIMESTAMP NOT NULL DEFAULT NOW(),
    blacklisted_by VARCHAR NOT NULL
);

-- Add index for efficient expiry cleanup
CREATE INDEX idx_agent_token_blacklist_expires_at ON agent_token_blacklist(expires_at);

-- Grant permissions to windmill users
GRANT ALL ON agent_token_blacklist TO windmill_user;
GRANT ALL ON agent_token_blacklist TO windmill_admin;