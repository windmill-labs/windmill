import process from "node:process";
import { existsSync } from "node:fs";

import { colors } from "@cliffy/ansi/colors";
import { Confirm } from "@cliffy/prompt/confirm";

import * as log from "../../core/log.ts";
import { GlobalOptions } from "../../types.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { readConfigFile } from "../../core/conf.ts";

import { ProtectionRulesConverter, ProtectionRulesPlan } from "./converter.ts";
import {
  getProtectionRulesPath,
  readProtectionRulesFile,
  WorkspaceResolver,
  configureClientForWorkspace,
} from "./file.ts";
import { outputResult, fail, displayPlan, structuredPlan } from "./utils.ts";

type PushOpts = GlobalOptions & {
  all?: boolean;
  dryRun?: boolean;
  jsonOutput?: boolean;
  yes?: boolean;
};

interface WsPlan {
  ws: string;
  wsId: string;
  plan: ProtectionRulesPlan;
  wipesAll: boolean;
}

export async function pushProtectionRules(
  opts: PushOpts,
  workspaceArg?: string,
) {
  // In JSON mode stdout must be exactly one JSON payload. Silence human logs
  // (log.info/warn → stdout, incl. workspace resolution + the empty-list
  // delete warning) before anything logs. log.error still goes to stderr.
  if (opts.jsonOutput) log.setSilent(true);

  const prPath = getProtectionRulesPath();
  if (!prPath) {
    fail(opts, {
      error:
        "No wmill.yaml found. Run 'wmill init' first — protection-rules.yaml lives next to it.",
    });
  }
  if (!existsSync(prPath!)) {
    fail(opts, {
      error:
        "No protection-rules.yaml found. Run 'wmill protection-rules pull' first.",
    });
  }

  const config = await readConfigFile();
  const resolver = WorkspaceResolver.fromConfig(config);
  const file = await readProtectionRulesFile(prPath!);

  let targets: string[];
  if (opts.all) {
    targets = Object.keys(file).sort();
    if (targets.length === 0) {
      fail(opts, { error: "protection-rules.yaml defines no workspaces." });
    }
  } else if (workspaceArg) {
    if (!(workspaceArg in file)) {
      fail(opts, {
        error: `Workspace '${workspaceArg}' is not defined in protection-rules.yaml.`,
      });
    }
    targets = [workspaceArg];
  } else {
    fail(opts, { error: "Specify a workspace name or use --all." });
  }

  // Phase 1: resolve + diff every target before mutating anything.
  const wsPlans: WsPlan[] = [];
  let hadError = false;
  for (const ws of targets) {
    let wsId: string;
    try {
      wsId = await configureClientForWorkspace(opts, ws, resolver);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (opts.all) {
        log.error(colors.red(msg));
        hadError = true;
        continue;
      }
      fail(opts, { error: msg });
    }

    let backend;
    try {
      backend = ProtectionRulesConverter.fromBackend(
        await wmill.listProtectionRules({ workspace: wsId }),
      );
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      if (opts.all) {
        log.error(colors.red(`[${ws}] failed to fetch: ${msg}`));
        hadError = true;
        continue;
      }
      fail(opts, { error: `Failed to fetch protection rules: ${msg}` });
    }

    const local = ProtectionRulesConverter.normalizeList(file[ws]);
    const plan = ProtectionRulesConverter.computePlan(local, backend);
    wsPlans.push({
      ws,
      wsId,
      plan,
      wipesAll: local.length === 0 && plan.toDelete.length > 0,
    });
  }

  const changed = wsPlans.filter((w) =>
    ProtectionRulesConverter.planHasChanges(w.plan)
  );

  if (opts.jsonOutput && opts.dryRun) {
    console.log(
      JSON.stringify({
        success: !hadError,
        dryRun: true,
        partialFailure: hadError,
        hasChanges: changed.length > 0,
        workspaces: Object.fromEntries(
          wsPlans.map((w) => [w.ws, structuredPlan(w.plan)]),
        ),
      }),
    );
    if (hadError) process.exit(1);
    return;
  }

  if (!opts.jsonOutput) {
    for (const w of wsPlans) displayPlan(w.ws, w.plan);
  }

  if (changed.length === 0) {
    if (hadError) {
      // A workspace failed to resolve/fetch — don't claim success while
      // exiting non-zero.
      outputResult(opts, {
        success: false,
        error:
          "One or more workspaces failed (see errors above); the rest are in sync",
        partialFailure: true,
      });
      process.exit(1);
    }
    if (!opts.dryRun) {
      outputResult(opts, {
        success: true,
        message: "No changes to push - all targeted workspaces are in sync",
      });
    }
    return;
  }

  if (opts.dryRun) {
    if (hadError) process.exit(1);
    return;
  }

  // Pushing an empty list wipes a workspace's rules — be loud even with --yes.
  for (const w of wsPlans) {
    if (w.wipesAll) {
      log.warn(
        colors.red(
          `WARNING: '${w.ws}' has an empty rule list — this DELETES ALL ${w.plan.toDelete.length} backend rule(s) for that workspace.`,
        ),
      );
    }
  }

  const totalDeletes = changed.reduce((n, w) => n + w.plan.toDelete.length, 0);
  if (!opts.yes && !!process.stdin.isTTY) {
    const confirmed = await Confirm.prompt({
      message: totalDeletes > 0
        ? `Apply these changes? This DELETES ${totalDeletes} protection rule(s) across ${changed.length} workspace(s).`
        : `Apply these changes to ${changed.length} workspace(s)?`,
      default: totalDeletes === 0,
    });
    if (!confirmed) {
      log.info("Operation cancelled");
      return;
    }
  }

  // Phase 2: apply. Track progress so a mid-run failure reports how far it got.
  const applied = { created: 0, updated: 0, deleted: 0 };
  try {
    for (const w of changed) {
      // Re-point the client at this workspace (phase 1 left it on the last one).
      await configureClientForWorkspace(opts, w.ws, resolver);
      for (const entry of w.plan.toCreate) {
        const n = ProtectionRulesConverter.normalizeEntry(entry);
        await wmill.createProtectionRule({
          workspace: w.wsId,
          requestBody: {
            name: n.name,
            rules: n.rules,
            bypass_groups: n.bypass_groups,
            bypass_users: n.bypass_users,
          },
        });
        applied.created++;
      }
      for (const entry of w.plan.toUpdate) {
        const n = ProtectionRulesConverter.normalizeEntry(entry);
        await wmill.updateProtectionRule({
          workspace: w.wsId,
          ruleName: n.name,
          requestBody: {
            rules: n.rules,
            bypass_groups: n.bypass_groups,
            bypass_users: n.bypass_users,
          },
        });
        applied.updated++;
      }
      for (const name of w.plan.toDelete) {
        await wmill.deleteProtectionRule({
          workspace: w.wsId,
          ruleName: name,
        });
        applied.deleted++;
      }
    }
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    fail(opts, {
      error:
        `Push partially failed after ${applied.created} create, ${applied.updated} update, ` +
        `${applied.deleted} delete: ${msg}. Backend is partially reconciled; re-run push to converge.`,
      applied,
    });
  }

  if (hadError) {
    // Reconcile of resolvable workspaces succeeded, but some --all targets
    // failed earlier. Status must reflect the non-zero exit.
    outputResult(opts, {
      success: false,
      error: `Pushed (created ${applied.created}, updated ${applied.updated}, deleted ${applied.deleted}) across ${changed.length} workspace(s), but one or more workspaces failed (see errors above)`,
      partialFailure: true,
      ...applied,
    });
    process.exit(1);
  }
  outputResult(opts, {
    success: true,
    message: `Pushed protection rules (created ${applied.created}, updated ${applied.updated}, deleted ${applied.deleted}) across ${changed.length} workspace(s)`,
    ...applied,
  });
}
