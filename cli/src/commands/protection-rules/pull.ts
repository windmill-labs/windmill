import { colors } from "@cliffy/ansi/colors";

import * as log from "../../core/log.ts";
import { GlobalOptions } from "../../types.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { readConfigFile } from "../../core/conf.ts";

import { ProtectionRulesConverter } from "./converter.ts";
import { ProtectionRulesFile } from "./types.ts";
import {
  PROTECTION_RULES_FILENAME,
  getProtectionRulesPath,
  readProtectionRulesFile,
  writeProtectionRulesFile,
  WorkspaceResolver,
  configureClientForWorkspace,
} from "./file.ts";
import { outputResult, fail, displayPlan, structuredPlan } from "./utils.ts";

type PullOpts = GlobalOptions & {
  all?: boolean;
  dryRun?: boolean;
  jsonOutput?: boolean;
};

export async function pullProtectionRules(
  opts: PullOpts,
  workspaceArg?: string,
) {
  // In JSON mode stdout must be exactly one JSON payload. Silence human logs
  // (log.info/warn → stdout) here, before anything that logs (readConfigFile,
  // workspace resolution). log.error still goes to stderr.
  if (opts.jsonOutput) log.setSilent(true);

  const prPath = getProtectionRulesPath();
  if (!prPath) {
    fail(opts, {
      error:
        "No wmill.yaml found. Run 'wmill init' first — protection-rules.yaml lives next to it.",
    });
  }

  const config = await readConfigFile();
  const resolver = WorkspaceResolver.fromConfig(config);

  let targets: string[];
  if (opts.all) {
    targets = resolver.knownNames();
    if (targets.length === 0) {
      fail(opts, {
        error: "No workspaces defined in wmill.yaml 'workspaces' block.",
      });
    }
  } else if (workspaceArg) {
    targets = [workspaceArg];
  } else {
    fail(opts, { error: "Specify a workspace name or use --all." });
  }

  const file = await readProtectionRulesFile(prPath!);
  const perWs: Record<string, any> = {};
  let hadError = false;
  let anyChange = false;

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

    const current = ProtectionRulesConverter.normalizeList(file[ws]);
    // plan describes how the local file would change to match the backend
    const plan = ProtectionRulesConverter.computePlan(backend, current);
    if (ProtectionRulesConverter.planHasChanges(plan)) anyChange = true;
    perWs[ws] = structuredPlan(plan);

    if (!opts.dryRun) {
      file[ws] = backend;
    } else if (!opts.jsonOutput) {
      displayPlan(ws, plan);
    }
  }

  if (opts.dryRun) {
    if (opts.jsonOutput) {
      console.log(
        JSON.stringify({
          success: !hadError,
          dryRun: true,
          partialFailure: hadError,
          hasChanges: anyChange,
          workspaces: perWs,
        }),
      );
    } else if (!hadError && !anyChange) {
      log.info(colors.green("All targeted workspaces are in sync"));
    }
    if (hadError) process.exit(1);
    return;
  }

  await writeProtectionRulesFile(prPath!, file as ProtectionRulesFile);
  const n = Object.keys(perWs).length;
  if (hadError) {
    // Some --all workspaces failed: status must not say success while we
    // exit non-zero.
    outputResult(opts, {
      success: false,
      error: `Pulled ${n} workspace(s) into ${PROTECTION_RULES_FILENAME}, but one or more workspaces failed (see errors above)`,
      partialFailure: true,
      workspaces: perWs,
    });
    process.exit(1);
  }
  outputResult(opts, {
    success: true,
    message: `Pulled protection rules for ${n} workspace(s) into ${PROTECTION_RULES_FILENAME}`,
    workspaces: perWs,
  });
}
