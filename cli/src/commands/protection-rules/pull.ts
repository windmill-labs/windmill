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
  fail,
  displayPlan,
  structuredLocalPlan,
  applyRulesToBranchOverride,
  clearRuleOverride,
  OverrideBlock,
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
      fail(opts, { error: `Failed to fetch protection rules: ${msg}` });
    }

    const wmillYamlPath = getWmillYamlPath();
    const wmillYamlExists = wmillYamlPath !== null;

    const localConfig: SyncOptions = wmillYamlExists
      ? await readConfigFile()
      : {};
    const currentSettings = wmillYamlExists
      ? await getEffectiveSettings(
          localConfig,
          opts.promotion,
          true,
          opts.jsonOutput,
        )
      : {};
    const currentRules = currentSettings.protectionRules;

    // Plan describes how the local file would change to match the backend.
    const plan = ProtectionRulesConverter.computePlan(backendRules, currentRules);
    const hasChanges = ProtectionRulesConverter.planHasChanges(plan);

    // --diff is a dry run: report and return BEFORE any write (including the
    // no-wmill.yaml bootstrap below), so it never mutates the working tree.
    if (opts.diff) {
      if (opts.jsonOutput) {
        console.log(
          JSON.stringify({
            success: true,
            hasChanges,
            local: ProtectionRulesConverter.normalizeList(currentRules),
            backend: backendRules,
            diff: structuredLocalPlan(plan),
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
      fail(opts, {
        error:
          "Protection rules conflict detected. Use --replace or --override to resolve.",
        hasConflict: true,
      });
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

    // The write target MUST match what getEffectiveSettings read above so the
    // pulled rules are actually effective. With --promotion it read the
    // promotion target's promotionOverrides; otherwise the current branch's
    // resolved entry overrides. findWorkspaceByGitBranch maps a branch to its
    // config key (which may differ, e.g. workspaces.prod.gitBranch=main).
    let resolvedWsKey: string | null;
    let overrideBlock: OverrideBlock;
    if (opts.promotion) {
      resolvedWsKey =
        findWorkspaceByGitBranch(localConfig.workspaces, opts.promotion)?.[0] ??
          opts.promotion;
      overrideBlock = "promotionOverrides";
    } else {
      const branch = isGitRepository() ? getCurrentGitBranch() : null;
      resolvedWsKey = branch
        ? findWorkspaceByGitBranch(localConfig.workspaces, branch)?.[0] ?? branch
        : null;
      overrideBlock = "overrides";
    }

    let updatedConfig: SyncOptions;
    if (writeMode === "replace") {
      updatedConfig = { ...localConfig, protectionRules: backendRules };
      if (resolvedWsKey) {
        if (!updatedConfig.workspaces) updatedConfig.workspaces = {} as any;
        if (!(updatedConfig.workspaces as any)[resolvedWsKey]) {
          (updatedConfig.workspaces as any)[resolvedWsKey] = {};
        }
        // An override (regular or promotion) would otherwise shadow the new
        // top-level value (getEffectiveSettings applies overrides last),
        // making replace a no-op and pull --diff loop forever.
        if (clearRuleOverride(updatedConfig, resolvedWsKey)) {
          log.info(
            colors.yellow(
              `Cleared a shadowing protectionRules override on workspace '${resolvedWsKey}' so the top-level value takes effect`,
            ),
          );
        }
      }
    } else {
      if (!resolvedWsKey) {
        fail(opts, {
          error:
            "--override requires a git repository with a current branch (or --promotion). Use --replace instead.",
        });
      }
      log.info(
        `Writing protection rules to workspace '${resolvedWsKey}' ${overrideBlock}`,
      );
      updatedConfig = applyRulesToBranchOverride(
        localConfig,
        resolvedWsKey,
        backendRules,
        overrideBlock,
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
          ? `Protection rules pulled into workspace '${resolvedWsKey}' ${overrideBlock}`
          : `Protection rules pulled into top-level configuration`,
      count: backendRules.length,
    });
  } catch (error) {
    const msg = error instanceof Error ? error.message : String(error);
    fail(opts, { error: `Failed to pull protection rules: ${msg}` });
  }
}
