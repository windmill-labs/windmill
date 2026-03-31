import { stat, writeFile, rm } from "node:fs/promises";
import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import { Confirm } from "@cliffy/prompt/confirm";
import * as log from "../../core/log.ts";
import { type BranchBinding } from "./template.ts";
import { GlobalOptions } from "../../types.ts";
import { readLockfile } from "../../utils/metadata.ts";
import { getActiveWorkspaceOrFallback } from "../workspace/workspace.ts";
import { generateRTNamespace } from "../resource-type/resource-type.ts";
import { writeAiGuidanceFiles } from "../../guidance/writer.ts";
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

const WMILL_INIT_AI_SKILLS_SOURCE = "WMILL_INIT_AI_SKILLS_SOURCE";
const WMILL_INIT_AI_AGENTS_SOURCE = "WMILL_INIT_AI_AGENTS_SOURCE";
const WMILL_INIT_AI_CLAUDE_SOURCE = "WMILL_INIT_AI_CLAUDE_SOURCE";

/**
 * Bootstrap a windmill project with a wmill.yaml file
 */
async function initAction(opts: InitOptions) {
  if (await stat("wmill.yaml").catch(() => null)) {
    log.info("wmill.yaml already exists, skipping config generation");
  } else {
    // Detect current git branch for template
    const { isGitRepository, getCurrentGitBranch } = await import(
      "../../utils/git.ts"
    );
    let branchName: string | undefined;
    let binding: BranchBinding | undefined;
    if (isGitRepository()) {
      branchName = getCurrentGitBranch() ?? undefined;
    }

    // Determine workspace binding before writing the template
    if (isGitRepository() && branchName) {
      const activeWorkspace = await getActiveWorkspaceOrFallback(
        opts as GlobalOptions
      );
      if (activeWorkspace) {
        const shouldBind = opts.bindProfile === true;
        const shouldPrompt =
          opts.bindProfile === undefined &&
          !!process.stdin.isTTY &&
          !opts.useDefault;
        const shouldSkip =
          opts.bindProfile != true &&
          (opts.useDefault || !process.stdin.isTTY);

        if (!shouldSkip) {
          if (shouldBind || shouldPrompt) {
            log.info(
              colors.yellow(`\nCurrent Git branch: ${colors.bold(branchName)}`)
            );
            log.info(
              colors.yellow(
                `Active workspace profile: ${colors.bold(activeWorkspace.name)}`
              )
            );
            log.info(
              colors.yellow(
                `  ${activeWorkspace.workspaceId} on ${activeWorkspace.remote}`
              )
            );
          }

          if (
            shouldBind ||
            (shouldPrompt &&
              (await Confirm.prompt({
                message: "Bind workspace profile to current Git branch?",
                default: true,
              })))
          ) {
            log.info(
              `binding branch ${branchName} to workspace ${activeWorkspace.name} on ${activeWorkspace.remote}`
            );
            binding = {
              baseUrl: activeWorkspace.remote,
              workspaceId: activeWorkspace.workspaceId,
            };
          }
        }
      }
    }

    await writeFile("wmill.yaml", generateCommentedTemplate(branchName, binding), "utf-8");
    log.info(colors.green("wmill.yaml created with default settings"));
    if (binding) {
      log.info(
        colors.green(
          `✓ Bound branch '${branchName}' to workspace`
        )
      );
    }

    // Create lock file
    await readLockfile();

    // Check for backend git-sync settings unless --use-default is specified
    if (!opts.useDefault) {
      try {
        const { requireLogin } = await import("../../core/auth.ts");
        const { resolveWorkspace } = await import("../../core/context.ts");

        // Check if user has workspace configured
        const { getActiveWorkspace } = await import(
          "../workspace/workspace.ts"
        );
        const activeWorkspace = await getActiveWorkspace(opts as GlobalOptions);

        if (!activeWorkspace) {
          log.info("No workspace configured. Using default settings.");
          log.info(
            "You can configure a workspace later with 'wmill workspace add'"
          );
        } else {
          await requireLogin(opts as GlobalOptions);
          const workspace = await resolveWorkspace(opts as GlobalOptions);

          const wmill = await import("../../../gen/services.gen.ts");
          const settings = await wmill.getSettings({
            workspace: workspace.workspaceId,
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
              // Interactive prompt
              const { Select } = await import("@cliffy/prompt/select");
              const choice = await Select.prompt({
                message:
                  "Git-sync settings found on backend. What would you like to do?",
                options: [
                  {
                    name: "Use backend git-sync settings",
                    value: "backend",
                  },
                  {
                    name: "Use default settings",
                    value: "default",
                  },
                  {
                    name: "Cancel",
                    value: "cancel",
                  },
                ],
              });

              if (choice === "cancel") {
                // Clean up the created files
                try {
                  await rm("wmill.yaml");
                  await rm("wmill-lock.yaml");
                } catch (e) {
                  // Ignore cleanup errors
                }
                log.info("Init cancelled");
                process.exit(0);
              }

              useBackendSettings = choice === "backend";
            }

            if (useBackendSettings) {
              log.info("Applying git-sync settings from backend...");

              // Import and run the pull git-sync settings logic
              const { pullGitSyncSettings } = await import(
                "../gitsync-settings/gitsync-settings.ts"
              );
              await pullGitSyncSettings({
                ...(opts as GlobalOptions),
                repository: opts.repository,
                jsonOutput: false,
                diff: false,
                replace: true, // Auto-replace when using backend settings during init
              });

              log.info(colors.green("Git-sync settings applied from backend"));
            }
          }
        }
      } catch (error) {
        // If there's an error checking backend settings, just continue with defaults
        log.warn(
          `Could not check backend for git-sync settings: ${(error as Error).message}`
        );
        log.info("Continuing with default settings");
      }
    }
  }

  // Read nonDottedPaths from config to specialize generated skills
  let nonDottedPaths = true; // default for new inits
  try {
    const { readConfigFile } = await import("../../core/conf.ts");
    const config = await readConfigFile();
    nonDottedPaths = config.nonDottedPaths ?? true;
  } catch {
    // If config can't be read, use default
  }

  // Create guidance files (AGENTS.md, CLAUDE.md, and Claude skills)
  try {
    const guidanceResult = await writeAiGuidanceFiles({
      targetDir: ".",
      nonDottedPaths,
      overwriteProjectGuidance: false,
      skillsSourcePath: process.env[WMILL_INIT_AI_SKILLS_SOURCE],
      agentsSourcePath: process.env[WMILL_INIT_AI_AGENTS_SOURCE],
      claudeSourcePath: process.env[WMILL_INIT_AI_CLAUDE_SOURCE],
    });

    if (guidanceResult.agentsWritten) {
      log.info(colors.green("Created AGENTS.md"));
    }
    if (guidanceResult.claudeWritten) {
      log.info(colors.green("Created CLAUDE.md"));
    }
    log.info(
      colors.green(`Created .claude/skills/ with ${guidanceResult.skillCount} skills`)
    );
  } catch (error) {
    if (error instanceof Error) {
      log.warn(`Could not create guidance files: ${error.message}`);
    } else {
      log.warn(`Could not create guidance files: ${error}`);
    }
  }

  // Generate resource type namespace
  try {
    await generateRTNamespace(opts as GlobalOptions);
  } catch (error) {
    log.warn(
      `Could not pull resource types and generate TypeScript namespace: ${
        error instanceof Error ? error.message : error
      }`
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
