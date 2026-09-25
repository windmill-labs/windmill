DROP TRIGGER IF EXISTS reset_agent_on_behalf_of ON resource;
DROP FUNCTION IF EXISTS reset_agent_on_behalf_of();
ALTER TABLE resource DROP COLUMN IF EXISTS on_behalf_of;
