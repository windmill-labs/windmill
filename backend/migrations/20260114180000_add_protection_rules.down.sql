-- Drop the workspace_protection_rule table and its indexes
DROP INDEX IF EXISTS idx_protection_rule_bypass_users;
DROP INDEX IF EXISTS idx_protection_rule_bypass_groups;
DROP INDEX IF EXISTS idx_protection_rule_workspace;
DROP TABLE IF EXISTS workspace_protection_rule;
