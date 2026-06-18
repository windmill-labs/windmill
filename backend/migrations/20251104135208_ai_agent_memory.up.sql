-- Add up migration script here

-- Create ai_agent_memory table for storing AI agent step memory when S3 is unavailable
CREATE TABLE ai_agent_memory (
    workspace_id VARCHAR(50) NOT NULL,
    conversation_id UUID NOT NULL,
    step_id VARCHAR(255) NOT NULL,
    messages JSONB NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    PRIMARY KEY (workspace_id, conversation_id, step_id)
);

-- Grant permissions
GRANT ALL ON ai_agent_memory TO windmill_admin;
GRANT ALL ON ai_agent_memory TO windmill_user;
