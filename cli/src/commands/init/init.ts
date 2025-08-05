import {
  colors,
  Command,
  log,
  yamlStringify,
  Confirm,
} from "../../../deps.ts";
import { GlobalOptions } from "../../types.ts";
import { readLockfile } from "../../utils/metadata.ts";
import { SCRIPT_GUIDANCE } from "../../guidance/script_guidance.ts";
import { FLOW_GUIDANCE } from "../../guidance/flow_guidance.ts";

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

/**
 * Bootstrap a windmill project with a wmill.yaml file
 */
async function initAction(opts: InitOptions) {
  if (await Deno.stat("wmill.yaml").catch(() => null)) {
    log.error(colors.red("wmill.yaml already exists"));
  } else {
    // Import DEFAULT_SYNC_OPTIONS from conf.ts
    const { DEFAULT_SYNC_OPTIONS } = await import("../../core/conf.ts");

    // Create initial config with defaults
    const initialConfig = { ...DEFAULT_SYNC_OPTIONS } as any;

    // Add branch structure
    const { isGitRepository, getCurrentGitBranch } = await import(
      "../../utils/git.ts"
    );
    if (isGitRepository()) {
      const currentBranch = getCurrentGitBranch();
      if (currentBranch) {
        initialConfig.git_branches = {
          [currentBranch]: { overrides: {} },
        };
      } else {
        initialConfig.git_branches = {};
      }
    } else {
      initialConfig.git_branches = {};
    }

    await Deno.writeTextFile("wmill.yaml", yamlStringify(initialConfig));
    log.info(colors.green("wmill.yaml created with default settings"));

    // Create lock file
    await readLockfile();

    // Offer to bind workspace profile to current branch
    if (isGitRepository()) {
      const { getActiveWorkspace } = await import("../workspace/workspace.ts");
      const activeWorkspace = await getActiveWorkspace(
        opts as GlobalOptions
      );
      const currentBranch = getCurrentGitBranch();

      if (activeWorkspace && currentBranch) {
        // Determine binding behavior based on flags
        const shouldBind = opts.bindProfile === true;
        const shouldPrompt =
          opts.bindProfile === undefined &&
          Deno.stdin.isTerminal() &&
          !opts.useDefault;
        const shouldSkip =
          opts.bindProfile === false ||
          opts.useDefault ||
          (!Deno.stdin.isTerminal() && opts.bindProfile === undefined);

        if (shouldSkip) {
          return;
        }

        // Show workspace info if we're binding or prompting
        if (shouldBind || shouldPrompt) {
          log.info(
            colors.yellow(
              `\nCurrent Git branch: ${colors.bold(currentBranch)}`
            )
          );
          log.info(
            colors.yellow(
              `Active workspace profile: ${colors.bold(
                activeWorkspace.name
              )}`
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
          // Update the config with workspace binding
          const currentConfig = await import("../../core/conf.ts").then((m) =>
            m.readConfigFile()
          );
          if (!currentConfig.git_branches) {
            currentConfig.git_branches = {};
          }
          if (!currentConfig.git_branches[currentBranch]) {
            currentConfig.git_branches[currentBranch] = { overrides: {} };
          }

          currentConfig.git_branches[currentBranch].baseUrl =
            activeWorkspace.remote;
          currentConfig.git_branches[currentBranch].workspaceId =
            activeWorkspace.workspaceId;

          await Deno.writeTextFile(
            "wmill.yaml",
            yamlStringify(currentConfig)
          );

          log.info(
            colors.green(
              `✓ Bound branch '${currentBranch}' to workspace '${activeWorkspace.name}'`
            )
          );
        }
      }
    }

    // Check for backend git-sync settings unless --use-default is specified
    if (!opts.useDefault) {
      try {
        const { requireLogin } = await import("../../core/auth.ts");
        const { resolveWorkspace } = await import("../../core/context.ts");

        // Check if user has workspace configured
        const { getActiveWorkspace } = await import("../workspace/workspace.ts");
        const activeWorkspace = await getActiveWorkspace(
          opts as GlobalOptions
        );

        if (!activeWorkspace) {
          log.info("No workspace configured. Using default settings.");
          log.info(
            "You can configure a workspace later with 'wmill workspace add'"
          );
          return;
        }

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
            const { Select } = await import("../../../deps.ts");
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
                await Deno.remove("wmill.yaml");
                await Deno.remove("wmill-lock.yaml");
              } catch (e) {
                // Ignore cleanup errors
              }
              log.info("Init cancelled");
              Deno.exit(0);
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

            log.info(
              colors.green("Git-sync settings applied from backend")
            );
          }
        }
      } catch (error) {
        // If there's an error checking backend settings, just continue with defaults
        log.warn(
          `Could not check backend for git-sync settings: ${error.message}`
        );
        log.info("Continuing with default settings");
      }
    }
  }

  // Create .cursor/rules directory and files with SCRIPT_GUIDANCE content
  try {
    const scriptGuidanceContent = SCRIPT_GUIDANCE;
    const flowGuidanceContent = FLOW_GUIDANCE;

    // Create .cursor/rules directory
    await Deno.mkdir(".cursor/rules", { recursive: true });

    // Create windmill.mdc file
    if (!(await Deno.stat(".cursor/rules/script.mdc").catch(() => null))) {
      await Deno.writeTextFile(
        ".cursor/rules/script.mdc",
        scriptGuidanceContent
      );
      log.info(colors.green("Created .cursor/rules/script.mdc"));
    }

    if (!(await Deno.stat(".cursor/rules/flow.mdc").catch(() => null))) {
      await Deno.writeTextFile(
        ".cursor/rules/flow.mdc",
        flowGuidanceContent
      );
      log.info(colors.green("Created .cursor/rules/flow.mdc"));
    }

    // Create CLAUDE.md file
    if (!(await Deno.stat("CLAUDE.md").catch(() => null))) {
      await Deno.writeTextFile(
        "CLAUDE.md",
        `
                        # Claude

                        You are a helpful assistant that can help with Windmill scripts and flows creation.

                        ## Script Guidance
                        ${scriptGuidanceContent}

                        ## Flow Guidance
                        ${flowGuidanceContent}
                    `
      );
      log.info(colors.green("Created CLAUDE.md"));
    }
  } catch (error) {
    if (error instanceof Error) {
      log.warn(`Could not create guidance files: ${error.message}`);
    } else {
      log.warn(`Could not create guidance files: ${error}`);
    }
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
