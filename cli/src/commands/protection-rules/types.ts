import { ProtectionRuleKind } from "../../../gen/types.gen.ts";

export type { ProtectionRuleKind };

// A single workspace protection ruleset as stored in protection-rules.yaml.
// Mirrors the backend ProtectionRuleset shape minus workspace_id (the workspace
// is the map key).
export interface ProtectionRuleEntry {
  name: string;
  rules: ProtectionRuleKind[];
  bypass_groups: string[];
  bypass_users: string[];
  // Role a member gets in a fork of the workspace, keyed by the role it holds there.
  fork_role_mapping?: Record<string, string>;
}

// protection-rules.yaml is a flat map: workspace name -> its protection rules.
// Workspace names MUST match keys in wmill.yaml's `workspaces` block, which is
// where the backend workspaceId/remote is resolved from.
export type ProtectionRulesFile = Record<string, ProtectionRuleEntry[]>;
