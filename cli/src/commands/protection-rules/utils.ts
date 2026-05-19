import process from "node:process";

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

// Report a genuine failure and exit non-zero so CI / scripted callers detect
// it. outputResult alone only logs, which would let `--yes` pushes mask a
// failed (possibly partial) reconcile behind exit code 0.
export function fail(
  opts: { jsonOutput?: boolean },
  result: { error: string; [key: string]: any },
): never {
  outputResult(opts, { ...result, success: false });
  process.exit(1);
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

// push direction: the plan describes operations applied to the backend.
export function structuredPlan(plan: ProtectionRulesPlan) {
  return {
    create: plan.toCreate.map((e) => e.name),
    update: plan.toUpdate.map((e) => e.name),
    delete: plan.toDelete,
    unchanged: plan.unchanged,
  };
}

// pull direction: the plan describes how the local wmill.yaml would change.
// Keys are named for the local-file delta to avoid ambiguity with the
// conventional create/delete framing used by push.
export function structuredLocalPlan(plan: ProtectionRulesPlan) {
  return {
    addedLocally: plan.toCreate.map((e) => e.name),
    updatedLocally: plan.toUpdate.map((e) => e.name),
    removedLocally: plan.toDelete,
    unchanged: plan.unchanged,
  };
}

export type OverrideBlock = "overrides" | "promotionOverrides";

// Write protectionRules into a workspace entry's override block, creating the
// entry if needed. `wsKey` MUST be the resolved workspace-config key (the key
// getEffectiveSettings/findWorkspaceByGitBranch would select) — NOT the raw
// git branch name. `block` MUST match the block getEffectiveSettings reads for
// the same invocation: `promotionOverrides` when --promotion is used,
// `overrides` otherwise. A mismatch leaves the written rules inert.
export function applyRulesToBranchOverride(
  config: SyncOptions,
  wsKey: string,
  rules: ProtectionRuleEntry[],
  block: OverrideBlock = "overrides",
): SyncOptions {
  const updated: SyncOptions = { ...config };
  if (!updated.workspaces) {
    updated.workspaces = {} as any;
  }
  if (!(updated.workspaces as any)[wsKey]) {
    (updated.workspaces as any)[wsKey] = {};
  }
  const entry = (updated.workspaces as any)[wsKey];
  if (!entry[block]) {
    entry[block] = {};
  }
  entry[block].protectionRules = rules;
  return updated;
}

// Remove any protectionRules override (regular + promotion) on a workspace
// entry so a top-level protectionRules value is no longer shadowed by it.
// Returns true if anything was cleared.
export function clearRuleOverride(
  config: SyncOptions,
  wsKey: string,
): boolean {
  const entry = (config.workspaces as any)?.[wsKey];
  if (!entry) return false;
  let cleared = false;
  for (const block of ["overrides", "promotionOverrides"] as const) {
    if (entry[block] && "protectionRules" in entry[block]) {
      delete entry[block].protectionRules;
      cleared = true;
    }
  }
  return cleared;
}
