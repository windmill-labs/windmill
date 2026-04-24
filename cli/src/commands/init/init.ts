import { stat, writeFile, rm, rmdir, readdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import { writeFileSync, mkdirSync, existsSync } from "node:fs";
import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import { Confirm } from "@cliffy/prompt/confirm";
import { Select } from "@cliffy/prompt/select";
import { Input } from "@cliffy/prompt/input";
import * as log from "../../core/log.ts";
import { type WorkspaceBinding } from "./template.ts";
import { GlobalOptions } from "../../types.ts";
import { readLockfile } from "../../utils/metadata.ts";
import {
  getActiveWorkspaceOrFallback,
  getActiveWorkspace,
  allWorkspaces,
  add as addWorkspaceProfile,
  type Workspace,
} from "../workspace/workspace.ts";
import { generateRTNamespace } from "../resource-type/resource-type.ts";
import {
  WMILL_INIT_AI_AGENTS_SOURCE_ENV,
  WMILL_INIT_AI_CLAUDE_SOURCE_ENV,
  WMILL_INIT_AI_SKILLS_SOURCE_ENV,
  writeAiGuidanceFiles,
} from "../../guidance/writer.ts";
import { generateCommentedTemplate } from "./template.ts";

export interface InitOptions {
  useDefault?: boolean;
  useBackend?: boolean;
  repository?: string;
  workspace?: string;
  debug?: unknown;
  showDiffs?: boolean;
  token?: string;
  baseUrl?: string;
  configDir?: string;
  bindProfile?: boolean;
}

const CLAUDE_MD_DEFAULT = "Instructions are in @AGENTS.md\n";

/**
 * Remove the Claude assets we generate (per `skipClaudeAssets: true` in wmill.yaml).
 * Narrow scope: only paths we'd otherwise have created. CLAUDE.md is left alone
 * if its content has been customized. .claude/ is removed only if it ends up empty.
 */
async function cleanupClaudeAssets(nonDottedPaths: boolean) {
  const flowSuffix = nonDottedPaths ? "__flow" : ".flow";
  const rawAppSuffix = nonDottedPaths ? "__raw_app" : ".raw_app";

  let introPrinted = false;
  function logRemoval(msg: string) {
    if (!introPrinted) {
      introPrinted = true;
      log.info(
        colors.gray(
          "skipClaudeAssets is true; removing previously-generated Claude assets:"
        )
      );
    }
    log.info(colors.yellow(`  ${msg}`));
  }

  async function tryRmEmptyDir(dir: string) {
    try {
      const remaining = await readdir(dir);
      if (remaining.length === 0) await rmdir(dir);
    } catch {
      /* dir already gone or not a dir */
    }
  }

  // Per-flow / per-raw_app .claude/launch.json
  async function scan(dir: string) {
    let entries;
    try {
      entries = await readdir(dir, { withFileTypes: true });
    } catch {
      return;
    }
    for (const entry of entries) {
      if (!entry.isDirectory()) continue;
      if (entry.name.startsWith(".") || entry.name === "node_modules") continue;
      const fullPath = join(dir, entry.name);
      if (entry.name.endsWith(flowSuffix) || entry.name.endsWith(rawAppSuffix)) {
        const claudeDir = join(fullPath, ".claude");
        const launchPath = join(claudeDir, "launch.json");
        if (existsSync(launchPath)) {
          await rm(launchPath);
          logRemoval(`Removed ${launchPath}`);
          await tryRmEmptyDir(claudeDir);
        }
      } else {
        await scan(fullPath);
      }
    }
  }
  await scan(".");

  // Root .claude/launch.json
  const rootLaunch = join(".claude", "launch.json");
  if (existsSync(rootLaunch)) {
    await rm(rootLaunch);
    logRemoval(`Removed ${rootLaunch}`);
  }

  // .claude/skills/ — wholly ours; safe to remove the subtree
  const skillsDir = join(".claude", "skills");
  if (existsSync(skillsDir)) {
    await rm(skillsDir, { recursive: true });
    logRemoval(`Removed ${skillsDir}/`);
  }

  await tryRmEmptyDir(".claude");

  // CLAUDE.md — only if untouched (content matches default). Otherwise warn.
  if (existsSync("CLAUDE.md")) {
    const content = await readFile("CLAUDE.md", "utf-8");
    if (content === CLAUDE_MD_DEFAULT) {
      await rm("CLAUDE.md");
      logRemoval("Removed CLAUDE.md");
    } else {
      logRemoval(
        "CLAUDE.md was customized; left in place. Delete manually if no longer needed."
      );
    }
  }
}

/**
 * Bootstrap a windmill project with a wmill.yaml file
 */
async function initAction(opts: InitOptions) {
  let didBindWorkspace = false;
  let boundProfile: Workspace | undefined;

  if (await stat("wmill.yaml").catch(() => null)) {
    log.info("wmill.yaml already exists, skipping config generation");
  } else {
    // Detect current git branch for template
    const { isGitRepository, getCurrentGitBranch } = await import(
      "../../utils/git.ts"
    );
    let branchName: string | undefined;
    let wsBindings: WorkspaceBinding[] | undefined;
    // boundProfile is set during bind (hoisted to function scope)
    const inGitRepo = isGitRepository();
    if (inGitRepo) {
      branchName = getCurrentGitBranch() ?? undefined;
    }

    const isInteractive = !!process.stdin.isTTY && !opts.useDefault;

    if (isInteractive && opts.bindProfile !== false) {
      const shouldBind = opts.bindProfile === true || await Confirm.prompt({
        message: "Bind a workspace?",
        default: true,
      });

      if (shouldBind) {
        // Step 1: Pick workspace profile (same as wmill workspace bind)
        let profiles = await allWorkspaces(opts.configDir);
        let selectedProfile: Workspace | undefined;

        if (profiles.length === 0) {
          log.info(colors.yellow("No workspace profiles found. Let's create one."));
          await addWorkspaceProfile(opts as any, undefined, undefined, undefined);
          profiles = await allWorkspaces(opts.configDir);
          selectedProfile = profiles.length > 0
            ? await getActiveWorkspace(opts as GlobalOptions)
            : undefined;
        } else {
          const activeProfile = await getActiveWorkspace(opts as GlobalOptions);
          const orderedProfiles = activeProfile
            ? [
                ...profiles.filter((p) => p.name === activeProfile.name),
                ...profiles.filter((p) => p.name !== activeProfile.name),
              ]
            : profiles;
          const selectedName = await Select.prompt({
            message: "Select workspace profile",
            options: orderedProfiles.map((p) => ({
              name: `${p.name} (${p.workspaceId} on ${p.remote})${
                activeProfile?.name === p.name ? " — active" : ""
              }`,
              value: p.name,
            })),
          });
          selectedProfile = profiles.find((p) => p.name === selectedName);
        }

        if (selectedProfile) {
          // Step 2: Pick workspace name
          const wsName = await Input.prompt({
            message: "Workspace name (key in wmill.yaml)",
            default: selectedProfile.workspaceId,
          });

          // Step 3: Pick git branch (only in git repos)
          let gitBranch: string | undefined;
          if (inGitRepo) {
            const branchInput = await Input.prompt({
              message: "Git branch to associate",
              default: branchName ?? wsName,
            });
            if (branchInput !== wsName) {
              gitBranch = branchInput;
            }
          }

          wsBindings = [{
            name: wsName,
            baseUrl: selectedProfile.remote,
            workspaceId: selectedProfile.workspaceId !== wsName ? selectedProfile.workspaceId : wsName,
            gitBranch,
          }];
          boundProfile = selectedProfile;
        }
      }
    } else if (opts.bindProfile === true) {
      // Non-interactive bind: create a single workspace entry from active profile
      const activeWorkspace = await getActiveWorkspaceOrFallback(
        opts as GlobalOptions
      );
      if (activeWorkspace) {
        const wsName = branchName ?? activeWorkspace.workspaceId;
        wsBindings = [{
          name: wsName,
          baseUrl: activeWorkspace.remote,
          workspaceId: activeWorkspace.workspaceId !== wsName ? activeWorkspace.workspaceId : wsName,
        }];
        boundProfile = activeWorkspace;
      }
    }

    await writeFile("wmill.yaml", generateCommentedTemplate(branchName, undefined, wsBindings), "utf-8");
    log.info(colors.green("wmill.yaml created with default settings"));
    if (wsBindings && wsBindings.length > 0) {
      didBindWorkspace = true;
      log.info(
        colors.green(
          `✓ Bound workspace '${wsBindings[0].name}' → ${wsBindings[0].workspaceId} on ${wsBindings[0].baseUrl}`
        )
      );
      log.info(
        colors.gray("To bind additional workspaces, run: wmill workspace bind")
      );
    } else {
      log.warn(
        "⚠️  No workspace bound. Sync commands will not work without a workspace.\n" +
        "   Run 'wmill workspace bind' to bind a workspace to this project."
      );
    }

    // Create lock file
    await readLockfile();

    // Check for backend git-sync settings — only if a workspace was bound and not --use-default
    if (!opts.useDefault && didBindWorkspace && boundProfile) {
      try {
        const { setClient } = await import("../../core/client.ts");

        // Use the bound profile directly — skip requireLogin which would resolve to the active profile
        setClient(boundProfile.token, boundProfile.remote.replace(/\/$/, ""));

        const wmill = await import("../../../gen/services.gen.ts");
        const settings = await wmill.getSettings({
          workspace: boundProfile.workspaceId,
        });

        if (
          settings.git_sync?.repositories &&
          settings.git_sync.repositories.length > 0
        ) {
          let useBackendSettings = opts.useBackend;

          // If repository is specified, implicitly use backend settings
          if (opts.repository && !opts.useDefault) {
            useBackendSettings = true;
          }

          if (useBackendSettings === undefined) {
            const choice = await Select.prompt({
              message:
                "Git-sync settings found on backend. What would you like to do?",
              options: [
                { name: "Use backend git-sync settings", value: "backend" },
                { name: "Use default settings", value: "default" },
                { name: "Cancel", value: "cancel" },
              ],
            });

            if (choice === "cancel") {
              try {
                await rm("wmill.yaml");
                await rm("wmill-lock.yaml");
              } catch {
                // Ignore cleanup errors
              }
              log.info("Init cancelled");
              process.exit(0);
            }

            useBackendSettings = choice === "backend";
          }

          if (useBackendSettings) {
            log.info("Applying git-sync settings from backend...");
            const { pullGitSyncSettings } = await import(
              "../gitsync-settings/gitsync-settings.ts"
            );
            const gsOpts = {
              ...(opts as GlobalOptions),
              workspace: boundProfile.name,
              repository: opts.repository,
              jsonOutput: false,
              diff: false,
              replace: true,
            };
            (gsOpts as any).__secret_workspace = boundProfile;
            await pullGitSyncSettings(gsOpts);
            log.info(colors.green("Git-sync settings applied from backend"));
          }
        }
      } catch (error) {
        log.warn(
          `Could not check backend for git-sync settings: ${(error as Error).message}`
        );
        log.info("Continuing with default settings");
      }
    }
  }

  // Read nonDottedPaths and skipClaudeAssets from config
  let nonDottedPaths = true; // default for new inits
  let skipClaudeAssets = false;
  try {
    const { readConfigFile } = await import("../../core/conf.ts");
    const config = await readConfigFile();
    nonDottedPaths = config.nonDottedPaths ?? true;
    skipClaudeAssets = config.skipClaudeAssets ?? false;
  } catch {
    // If config can't be read, use defaults
  }

  // Create guidance files (AGENTS.md, plus CLAUDE.md and Claude skills unless skipClaudeAssets)
  try {
    const guidanceResult = await writeAiGuidanceFiles({
      targetDir: ".",
      nonDottedPaths,
      overwriteProjectGuidance: false,
      skillsSourcePath: process.env[WMILL_INIT_AI_SKILLS_SOURCE_ENV],
      agentsSourcePath: process.env[WMILL_INIT_AI_AGENTS_SOURCE_ENV],
      claudeSourcePath: process.env[WMILL_INIT_AI_CLAUDE_SOURCE_ENV],
      skipClaudeAssets,
    });

    if (guidanceResult.agentsWritten) {
      log.info(colors.green("Created AGENTS.md"));
    }
    if (guidanceResult.claudeWritten) {
      log.info(colors.green("Created CLAUDE.md"));
    }
    if (guidanceResult.skillCount > 0) {
      log.info(
        colors.green(`Created .claude/skills/ with ${guidanceResult.skillCount} skills`)
      );
    }
  } catch (error) {
    if (error instanceof Error) {
      log.warn(`Could not create guidance files: ${error.message}`);
    } else {
      log.warn(`Could not create guidance files: ${error}`);
    }
  }

  if (skipClaudeAssets) {
    await cleanupClaudeAssets(nonDottedPaths);
  }

  if (!skipClaudeAssets) {
  // Generate .claude/launch.json at the workspace root so Claude Code can launch
  // `wmill dev` from there and land on the file picker (no --path → picker mode).
  try {
    const rootClaudeDir = join(".", ".claude");
    const rootLaunchPath = join(rootClaudeDir, "launch.json");
    if (existsSync(rootLaunchPath)) {
      log.info(colors.gray(`Skipped ${rootLaunchPath} (already exists)`));
    } else {
      const rootLaunchJson = JSON.stringify({
        version: "0.0.1",
        configurations: [{
          name: "windmill",
          runtimeExecutable: "bash",
          runtimeArgs: ["-c", "wmill dev --proxy-port ${PORT:-4000} --no-open"],
          port: 4000,
          autoPort: true,
        }],
      }, null, 2) + "\n";
      mkdirSync(rootClaudeDir, { recursive: true });
      writeFileSync(rootLaunchPath, rootLaunchJson, "utf-8");
      log.info(colors.green(`Created ${rootLaunchPath}`));
    }
  } catch (error) {
    log.warn(
      `Could not create root .claude/launch.json: ${error instanceof Error ? error.message : error}`
    );
  }

  // Generate .claude/launch.json for each flow folder
  try {
    const flowSuffix = nonDottedPaths ? "__flow" : ".flow";
    const flowLaunchJson = JSON.stringify({
      version: "0.0.1",
      configurations: [{
        name: "windmill",
        runtimeExecutable: "bash",
        runtimeArgs: ["-c", "wmill dev --proxy-port ${PORT:-4000} --no-open"],
        port: 4000,
        autoPort: true,
      }],
    }, null, 2) + "\n";

    let flowCount = 0;
    async function scanForFlows(dir: string) {
      const entries = await readdir(dir, { withFileTypes: true });
      for (const entry of entries) {
        if (!entry.isDirectory()) continue;
        if (entry.name.startsWith(".") || entry.name === "node_modules") continue;
        const fullPath = join(dir, entry.name);
        if (entry.name.endsWith(flowSuffix)) {
          const claudeDir = join(fullPath, ".claude");
          const launchPath = join(claudeDir, "launch.json");
          mkdirSync(claudeDir, { recursive: true });
          writeFileSync(launchPath, flowLaunchJson, "utf-8");
          flowCount++;
        } else {
          await scanForFlows(fullPath);
        }
      }
    }

    await scanForFlows(".");
    if (flowCount > 0) {
      log.info(colors.green(`Created .claude/launch.json for ${flowCount} flow folder(s)`));
    }
  } catch (error) {
    log.warn(
      `Could not scan for flow folders: ${error instanceof Error ? error.message : error}`
    );
  }

  // Generate .claude/launch.json for each raw_app folder
  try {
    const rawAppSuffix = nonDottedPaths ? "__raw_app" : ".raw_app";
    const appLaunchJson = JSON.stringify({
      version: "0.0.1",
      configurations: [{
        name: "windmill",
        runtimeExecutable: "bash",
        runtimeArgs: ["-c", "wmill app dev --no-open --port ${PORT:-4000}"],
        port: 4000,
        autoPort: true,
      }],
    }, null, 2) + "\n";

    let appCount = 0;
    async function scanForApps(dir: string) {
      const entries = await readdir(dir, { withFileTypes: true });
      for (const entry of entries) {
        if (!entry.isDirectory()) continue;
        if (entry.name.startsWith(".") || entry.name === "node_modules") continue;
        const fullPath = join(dir, entry.name);
        if (entry.name.endsWith(rawAppSuffix)) {
          const claudeDir = join(fullPath, ".claude");
          const launchPath = join(claudeDir, "launch.json");
          mkdirSync(claudeDir, { recursive: true });
          writeFileSync(launchPath, appLaunchJson, "utf-8");
          appCount++;
        } else {
          await scanForApps(fullPath);
        }
      }
    }

    await scanForApps(".");
    if (appCount > 0) {
      log.info(colors.green(`Created .claude/launch.json for ${appCount} raw app folder(s)`));
    }
  } catch (error) {
    log.warn(
      `Could not scan for raw app folders: ${error instanceof Error ? error.message : error}`
    );
  }
  } // end if (!skipClaudeAssets)

  // Generate resource type namespace (only if a workspace was bound)
  if (didBindWorkspace && boundProfile) {
    try {
      // Cache the bound profile so resolveWorkspace doesn't re-resolve and prompt again
      const rtOpts = { ...opts } as GlobalOptions;
      (rtOpts as any).__secret_workspace = boundProfile;
      await generateRTNamespace(rtOpts);
    } catch (error) {
      log.warn(
        `Could not pull resource types and generate TypeScript namespace: ${
          error instanceof Error ? error.message : error
        }`
      );
    }
  } else {
    log.info(
      colors.gray("Skipped resource type namespace generation (no workspace bound). Run 'wmill workspace bind' then 'wmill init' to generate it.")
    );
  }
}

const command = new Command()
  .description("Bootstrap a windmill project with a wmill.yaml file")
  .option("--use-default", "Use default settings without checking backend")
  .option("--use-backend", "Use backend git-sync settings if available")
  .option(
    "--repository <repo:string>",
    "Specify repository path (e.g., u/user/repo) when using backend settings"
  )
  .option(
    "--bind-profile",
    "Automatically bind active workspace profile to current Git branch"
  )
  .option("--no-bind-profile", "Skip workspace profile binding prompt")
  .action(initAction as any);

export default command;
