import { writeFile } from "node:fs/promises";
import { colors } from "@cliffy/ansi/colors";
import { stringify as yamlStringify } from "yaml";

import * as log from "../../core/log.ts";
import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import * as wmill from "../../../gen/services.gen.ts";
import {
  SyncOptions,
  ProtectionRuleEntry,
  readConfigFile,
  getEffectiveSettings,
  getWmillYamlPath,
  findWorkspaceByGitBranch,
} from "../../core/conf.ts";
import { yamlOptions } from "../sync/sync.ts";
import { getCurrentGitBranch, isGitRepository } from "../../utils/git.ts";

import { ProtectionRulesConverter } from "./converter.ts";
import {
  outputResult,
  displayPlan,
  structuredPlan,
  applyRulesToBranchOverride,
} from "./utils.ts";

type WriteMode = "replace" | "override";

export async function pullProtectionRules(
  opts: GlobalOptions & {
    default?: boolean;
    replace?: boolean;
    override?: boolean;
    diff?: boolean;
    jsonOutput?: boolean;
    yes?: boolean;
    promotion?: string;
  },
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  try {
    let backendRules: ProtectionRuleEntry[];
    try {
      const response = await wmill.listProtectionRules({
        workspace: workspace.workspaceId,
      });
      backendRules = ProtectionRulesConverter.fromBackend(response);
    } catch (apiError) {
      const msg = apiError instanceof Error ? apiError.message : String(apiError);
      outputResult(opts, {
        success: false,
        error: `Failed to fetch protection rules: ${msg}`,
      });
      return;
    }

    const wmillYamlPath = getWmillYamlPath();
    const wmillYamlExists = wmillYamlPath !== null;

    // No wmill.yaml yet: create one carrying the backend protection rules.
    if (!wmillYamlExists) {
      const newConfig: SyncOptions = { protectionRules: backendRules };
      if (isGitRepository()) {
        const branch = getCurrentGitBranch();
        if (branch) {
          newConfig.workspaces = { [branch]: {} } as any;
        }
      }
      await writeFile(
        "wmill.yaml",
        yamlStringify(newConfig, yamlOptions),
        "utf-8",
      );
      outputResult(opts, {
        success: true,
        message: "wmill.yaml created with backend protection rules",
        count: backendRules.length,
      });
      return;
    }

    const localConfig = await readConfigFile();
    const currentSettings = await getEffectiveSettings(
      localConfig,
      opts.promotion,
      true,
      opts.jsonOutput,
    );
    const currentRules = currentSettings.protectionRules;

    // Plan describes how the local file would change to match the backend.
    const plan = ProtectionRulesConverter.computePlan(backendRules, currentRules);
    const hasChanges = ProtectionRulesConverter.planHasChanges(plan);

    if (opts.diff) {
      if (opts.jsonOutput) {
        console.log(
          JSON.stringify({
            success: true,
            hasChanges,
            local: ProtectionRulesConverter.normalizeList(currentRules),
            backend: backendRules,
            diff: structuredPlan(plan),
          }),
        );
      } else if (hasChanges) {
        log.info("Changes that would be applied locally:");
        displayPlan(plan);
      } else {
        log.info(colors.green("No differences found"));
      }
      return;
    }

    let writeMode: WriteMode;
    if (opts.default || opts.replace) {
      writeMode = "replace";
    } else if (opts.override) {
      writeMode = "override";
    } else if (!hasChanges) {
      writeMode = "replace";
    } else if (!opts.yes && !!process.stdin.isTTY) {
      log.info("Changes that would be applied locally:");
      displayPlan(plan);
      const { Select } = await import("@cliffy/prompt/select");
      const choice = await Select.prompt({
        message: "How would you like to write the protection rules?",
        options: [
          { name: "Replace top-level protection rules", value: "replace" },
          { name: "Add branch-specific override", value: "override" },
          { name: "Cancel", value: "cancel" },
        ],
      });
      if (choice === "cancel") {
        log.info("Operation cancelled");
        return;
      }
      writeMode = choice as WriteMode;
    } else if (opts.yes) {
      writeMode = "override";
      log.info(
        colors.yellow(
          "Conflict detected. Using --override behavior (default for --yes).",
        ),
      );
    } else {
      outputResult(opts, {
        success: false,
        error:
          "Protection rules conflict detected. Use --replace or --override to resolve.",
        hasConflict: true,
      });
      return;
    }

    if (!hasChanges) {
      // Even with no changes, ensure an empty branch structure exists so
      // overrides have a home, mirroring gitsync-settings behavior.
      if (isGitRepository()) {
        const branch = getCurrentGitBranch();
        if (
          branch &&
          (!localConfig.workspaces ||
            !findWorkspaceByGitBranch(localConfig.workspaces, branch))
        ) {
          const updated: SyncOptions = { ...localConfig };
          if (!updated.workspaces) updated.workspaces = {} as any;
          if (!(updated.workspaces as any)[branch]) {
            (updated.workspaces as any)[branch] = {};
          }
          await writeFile(
            "wmill.yaml",
            yamlStringify(updated, yamlOptions),
            "utf-8",
          );
          outputResult(opts, {
            success: true,
            message: `Created empty branch structure for: ${branch}`,
          });
          return;
        }
      }
      outputResult(opts, {
        success: true,
        message: "No changes needed - protection rules are already up to date",
      });
      return;
    }

    let updatedConfig: SyncOptions;
    if (writeMode === "replace") {
      updatedConfig = { ...localConfig, protectionRules: backendRules };
      if (isGitRepository()) {
        const branch = getCurrentGitBranch();
        if (branch) {
          if (!updatedConfig.workspaces) updatedConfig.workspaces = {} as any;
          if (!(updatedConfig.workspaces as any)[branch]) {
            (updatedConfig.workspaces as any)[branch] = {};
          }
        }
      }
    } else {
      const branch = isGitRepository() ? getCurrentGitBranch() : null;
      if (!branch) {
        outputResult(opts, {
          success: false,
          error:
            "--override requires a git repository with a current branch. Use --replace instead.",
        });
        return;
      }
      log.info(`Writing protection rules to branch override: ${branch}`);
      updatedConfig = applyRulesToBranchOverride(
        localConfig,
        branch,
        backendRules,
      );
    }

    await writeFile(
      "wmill.yaml",
      yamlStringify(updatedConfig, yamlOptions),
      "utf-8",
    );
    outputResult(opts, {
      success: true,
      message:
        writeMode === "override"
          ? `Protection rules pulled into branch override`
          : `Protection rules pulled into top-level configuration`,
      count: backendRules.length,
    });
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    outputResult(opts, {
      success: false,
      error: `Failed to pull protection rules: ${msg}`,
    });
  }
}
