import process from "node:process";

import { colors } from "@cliffy/ansi/colors";
import { Confirm } from "@cliffy/prompt/confirm";

import * as log from "../../core/log.ts";
import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import * as wmill from "../../../gen/services.gen.ts";
import {
  ProtectionRuleEntry,
  readConfigFile,
  validateBranchConfiguration,
  getEffectiveSettings,
  getWmillYamlPath,
} from "../../core/conf.ts";

import { ProtectionRulesConverter } from "./converter.ts";
import { outputResult, fail, displayPlan, structuredPlan } from "./utils.ts";

export async function pushProtectionRules(
  opts: GlobalOptions & {
    diff?: boolean;
    jsonOutput?: boolean;
    yes?: boolean;
    promotion?: string;
  },
) {
  try {
    await validateBranchConfiguration({ yes: opts.yes });
  } catch (error) {
    if (error instanceof Error && error.message.includes("overrides")) {
      log.error(error.message);
      process.exit(1);
    }
    throw error;
  }

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  try {
    const wmillYamlPath = getWmillYamlPath();
    if (!wmillYamlPath) {
      fail(opts, {
        error:
          "No wmill.yaml file found. Run 'wmill protection-rules pull' or 'wmill init' first.",
      });
    }

    const localConfig = await readConfigFile();
    const effectiveSettings = await getEffectiveSettings(
      localConfig,
      opts.promotion,
      true,
      opts.jsonOutput,
    );
    const localRules: ProtectionRuleEntry[] | undefined =
      effectiveSettings.protectionRules;

    if (localRules === undefined) {
      fail(opts, {
        error:
          "No protectionRules defined in wmill.yaml. Nothing to push (use 'pull' first to seed them).",
      });
    }

    let backendRules: ProtectionRuleEntry[];
    try {
      const response = await wmill.listProtectionRules({
        workspace: workspace.workspaceId,
      });
      backendRules = ProtectionRulesConverter.fromBackend(response);
    } catch (apiError) {
      const msg = apiError instanceof Error ? apiError.message : String(apiError);
      fail(opts, { error: `Failed to fetch protection rules: ${msg}` });
    }

    // Full reconcile: backend must end up matching wmill.yaml exactly.
    const plan = ProtectionRulesConverter.computePlan(localRules, backendRules);
    const hasChanges = ProtectionRulesConverter.planHasChanges(plan);

    if (opts.diff) {
      if (opts.jsonOutput) {
        console.log(
          JSON.stringify({
            success: true,
            hasChanges,
            local: ProtectionRulesConverter.normalizeList(localRules),
            backend: backendRules,
            diff: structuredPlan(plan),
          }),
        );
      } else if (hasChanges) {
        log.info("Changes that would be pushed to Windmill:");
        displayPlan(plan);
      } else {
        log.info(colors.green("No changes to push"));
      }
      return;
    }

    if (!hasChanges) {
      outputResult(opts, {
        success: true,
        message: "No changes to push - protection rules are already in sync",
      });
      return;
    }

    if (!opts.jsonOutput) {
      log.info("Changes that would be pushed to Windmill:");
      displayPlan(plan);
    }

    // Pushing an empty protectionRules wipes every backend rule under
    // full-reconcile semantics — loud about it even in --yes/non-TTY runs.
    const wipesAll =
      localRules.length === 0 && plan.toDelete.length > 0;
    if (wipesAll) {
      log.warn(
        colors.red(
          `WARNING: wmill.yaml defines an empty protectionRules list — this will DELETE ALL ${plan.toDelete.length} protection rule(s) on the backend.`,
        ),
      );
    }

    if (!opts.yes && !!process.stdin.isTTY) {
      const confirmed = await Confirm.prompt({
        message: plan.toDelete.length > 0
          ? `This will DELETE ${plan.toDelete.length} protection rule(s) on the backend. Continue?`
          : `Apply these changes to the remote?`,
        default: plan.toDelete.length === 0,
      });
      if (!confirmed) {
        log.info("Operation cancelled");
        return;
      }
    }

    // The reconcile is N independent API calls with no transaction. Track
    // applied ops so a mid-loop failure reports how far it got (the backend
    // is left partially reconciled; re-running push is idempotent).
    const ws = workspace.workspaceId;
    const applied = { created: 0, updated: 0, deleted: 0 };
    try {
      for (const entry of plan.toCreate) {
        const n = ProtectionRulesConverter.normalizeEntry(entry);
        await wmill.createProtectionRule({
          workspace: ws,
          requestBody: {
            name: n.name,
            rules: n.rules,
            bypass_groups: n.bypass_groups,
            bypass_users: n.bypass_users,
          },
        });
        applied.created++;
      }
      for (const entry of plan.toUpdate) {
        const n = ProtectionRulesConverter.normalizeEntry(entry);
        await wmill.updateProtectionRule({
          workspace: ws,
          ruleName: n.name,
          requestBody: {
            rules: n.rules,
            bypass_groups: n.bypass_groups,
            bypass_users: n.bypass_users,
          },
        });
        applied.updated++;
      }
      for (const name of plan.toDelete) {
        await wmill.deleteProtectionRule({ workspace: ws, ruleName: name });
        applied.deleted++;
      }
    } catch (applyError) {
      const msg =
        applyError instanceof Error ? applyError.message : String(applyError);
      fail(opts, {
        error:
          `Push partially failed after applying ${applied.created}/${plan.toCreate.length} create, ` +
          `${applied.updated}/${plan.toUpdate.length} update, ${applied.deleted}/${plan.toDelete.length} delete: ${msg}. ` +
          `The backend is partially reconciled; re-run push to converge.`,
        applied,
      });
    }

    outputResult(opts, {
      success: true,
      message: `Protection rules pushed successfully (created ${applied.created}, updated ${applied.updated}, deleted ${applied.deleted})`,
      created: applied.created,
      updated: applied.updated,
      deleted: applied.deleted,
    });
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    fail(opts, { error: `Failed to push protection rules: ${msg}` });
  }
}
