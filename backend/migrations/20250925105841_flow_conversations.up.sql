-- Add up migration script here

-- Create message_type enum
CREATE TYPE MESSAGE_TYPE AS ENUM ('user', 'assistant');

-- Create flow_conversation table
CREATE TABLE flow_conversation (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    workspace_id VARCHAR(50) NOT NULL REFERENCES workspace(id),
    flow_path VARCHAR(255) NOT NULL,
    title VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    created_by VARCHAR(50) NOT NULL
);

-- Create flow_conversation_message table
CREATE TABLE flow_conversation_message (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id UUID NOT NULL REFERENCES flow_conversation(id) ON DELETE CASCADE,
    message_type MESSAGE_TYPE NOT NULL,
    content TEXT NOT NULL,
    job_id UUID REFERENCES v2_job(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Basic indexes for performance
CREATE INDEX idx_flow_conversation_workspace_path ON flow_conversation(workspace_id, flow_path, updated_at DESC);
CREATE INDEX idx_conversation_message_conversation_time ON flow_conversation_message(conversation_id, created_at DESC);

-- Grant permissions
GRANT ALL ON flow_conversation TO windmill_admin;
GRANT ALL ON flow_conversation TO windmill_user;
GRANT ALL ON flow_conversation_message TO windmill_admin;
GRANT ALL ON flow_conversation_message TO windmill_user;

-- RLS policies
ALTER TABLE flow_conversation ENABLE ROW LEVEL SECURITY;
ALTER TABLE flow_conversation_message ENABLE ROW LEVEL SECURITY;

-- Admin policies - admins can access all conversations
CREATE POLICY admin_policy ON flow_conversation FOR ALL TO windmill_admin USING (true);
CREATE POLICY admin_policy ON flow_conversation_message FOR ALL TO windmill_admin USING (true);

-- User policies - users can only access their own conversations
CREATE POLICY see_own ON flow_conversation FOR ALL TO windmill_user
USING (flow_conversation.created_by = current_setting('session.user'));

-- Users can see messages of conversations they own
CREATE POLICY see_own ON flow_conversation_message FOR ALL TO windmill_user
USING (
    EXISTS (
        SELECT 1 FROM flow_conversation 
        WHERE flow_conversation.id = flow_conversation_message.conversation_id 
        AND flow_conversation.created_by = current_setting('session.user')
    )
);