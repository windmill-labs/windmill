-- Drop the workspace_protection_rule table and its indexes
DROP TRIGGER IF EXISTS update_workspace_protection_rule_updated_at ON workspace_protection_rule;
DROP INDEX IF EXISTS idx_protection_rule_bypass_users;
DROP INDEX IF EXISTS idx_protection_rule_bypass_groups;
DROP INDEX IF EXISTS idx_protection_rule_workspace;
DROP TABLE IF EXISTS workspace_protection_rule;
