import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { ProtectionRuleEntry, SyncOptions } from "../../core/conf.ts";
import { ProtectionRulesConverter, ProtectionRulesPlan } from "./converter.ts";

export function outputResult(
  opts: { jsonOutput?: boolean },
  result: {
    success: boolean;
    message?: string;
    error?: string;
    [key: string]: any;
  },
): void {
  if (opts.jsonOutput) {
    console.log(JSON.stringify(result));
  } else if (result.success && result.message) {
    log.info(colors.green(result.message));
  } else if (!result.success && result.error) {
    log.error(colors.red(result.error));
  }
}

function describeEntry(entry: ProtectionRuleEntry): string {
  const n = ProtectionRulesConverter.normalizeEntry(entry);
  const parts = [`rules=[${n.rules.join(", ")}]`];
  if (n.bypass_groups.length > 0) {
    parts.push(`bypass_groups=[${n.bypass_groups.join(", ")}]`);
  }
  if (n.bypass_users.length > 0) {
    parts.push(`bypass_users=[${n.bypass_users.join(", ")}]`);
  }
  return parts.join(" ");
}

// Render a reconciliation plan. `verb` differs by direction:
// pull applies changes locally, push applies them to the backend.
export function displayPlan(plan: ProtectionRulesPlan): void {
  for (const e of plan.toCreate) {
    log.info(colors.green(`  + ${e.name}  (${describeEntry(e)})`));
  }
  for (const e of plan.toUpdate) {
    log.info(colors.yellow(`  ~ ${e.name}  (${describeEntry(e)})`));
  }
  for (const name of plan.toDelete) {
    log.info(colors.red(`  - ${name}`));
  }
  if (!ProtectionRulesConverter.planHasChanges(plan)) {
    log.info(colors.green("  No differences found"));
  }
}

export function structuredPlan(plan: ProtectionRulesPlan) {
  return {
    create: plan.toCreate.map((e) => e.name),
    update: plan.toUpdate.map((e) => e.name),
    delete: plan.toDelete,
    unchanged: plan.unchanged,
  };
}

// Write protectionRules into a branch/workspace override block, creating the
// workspace entry if needed. Mirrors the override mechanics used by
// gitsync-settings but scoped to the single protectionRules field.
export function applyRulesToBranchOverride(
  config: SyncOptions,
  branchName: string,
  rules: ProtectionRuleEntry[],
): SyncOptions {
  const updated: SyncOptions = { ...config };
  if (!updated.workspaces) {
    updated.workspaces = {} as any;
  }
  if (!(updated.workspaces as any)[branchName]) {
    (updated.workspaces as any)[branchName] = {};
  }
  const entry = (updated.workspaces as any)[branchName];
  if (!entry.overrides) {
    entry.overrides = {};
  }
  entry.overrides.protectionRules = rules;
  return updated;
}
