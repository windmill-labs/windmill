import process from "node:process";

import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { ProtectionRuleEntry } from "./types.ts";
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
// it. outputResult alone only logs, which would let a failed (possibly
// partial) reconcile hide behind exit code 0.
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

// Render a reconciliation plan with a per-workspace heading.
export function displayPlan(ws: string, plan: ProtectionRulesPlan): void {
  log.info(colors.bold(`workspace ${ws}:`));
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
    log.info(colors.green("  in sync"));
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
