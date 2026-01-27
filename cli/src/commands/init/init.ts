import { colors, Command, log, yamlStringify, Confirm } from "../../../deps.ts";
import { GlobalOptions } from "../../types.ts";
import { readLockfile } from "../../utils/metadata.ts";
import { getActiveWorkspaceOrFallback } from "../workspace/workspace.ts";
import { generateRTNamespace } from "../resource-type/resource-type.ts";
import { SCRIPT_BASE, FLOW_BASE, CLI_COMMANDS } from "../../guidance/prompts.ts";
import { SKILLS, SKILL_CONTENT } from "../../guidance/skills.ts";

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
        initialConfig.gitBranches = {
          [currentBranch]: { overrides: {} },
        };
      } else {
        initialConfig.gitBranches = {};
      }
    } else {
      initialConfig.gitBranches = {};
    }

    initialConfig.nonDottedPaths = true;
    await Deno.writeTextFile("wmill.yaml", yamlStringify(initialConfig));
    log.info(colors.green("wmill.yaml created with default settings"));

    // Create lock file
    await readLockfile();

    // Offer to bind workspace profile to current branch
    if (isGitRepository()) {
      const activeWorkspace = await getActiveWorkspaceOrFallback(
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
          opts.bindProfile != true &&
          (opts.useDefault || !Deno.stdin.isTerminal());

        if (shouldSkip) {
          return;
        }

        // Show workspace info if we're binding or prompting
        if (shouldBind || shouldPrompt) {
          log.info(
            colors.yellow(`\nCurrent Git branch: ${colors.bold(currentBranch)}`)
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
          // Update the config with workspace binding
          const currentConfig = await import("../../core/conf.ts").then((m) =>
            m.readConfigFile()
          );
          if (!currentConfig.gitBranches) {
            currentConfig.gitBranches = {};
          }
          if (!currentConfig.gitBranches[currentBranch]) {
            currentConfig.gitBranches[currentBranch] = { overrides: {} };
          }

          log.info(
            `binding branch ${currentBranch} to workspace ${activeWorkspace.name} on ${activeWorkspace.remote}`
          );
          currentConfig.gitBranches[currentBranch].baseUrl =
            activeWorkspace.remote;
          currentConfig.gitBranches[currentBranch].workspaceId =
            activeWorkspace.workspaceId;

          await Deno.writeTextFile("wmill.yaml", yamlStringify(currentConfig));

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
        const { getActiveWorkspace } = await import(
          "../workspace/workspace.ts"
        );
        const activeWorkspace = await getActiveWorkspace(opts as GlobalOptions);

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

            log.info(colors.green("Git-sync settings applied from backend"));
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

  // Create guidance files (AGENTS.md, CLAUDE.md, and Claude skills)
  try {
    // Generate skills reference section for AGENTS.md
    const skills_base_dir = ".claude/skills";
    const skillsReference = SKILLS.map(
      (s) => `- \`${skills_base_dir}/${s.name}/SKILL.md\` - ${s.description}`
    ).join("\n");

    // Create AGENTS.md file with general instructions only
    if (!(await Deno.stat("AGENTS.md").catch(() => null))) {
      await Deno.writeTextFile(
        "AGENTS.md",
        `# Windmill AI Agent Instructions

You are a helpful assistant that can help with Windmill scripts and flows creation.

## Getting Started

Scripts and flows should be placed in folders. After writing:
- \`wmill script generate-metadata\` - Generate .script.yaml and .lock files for scripts
- \`wmill flow generate-locks --yes\` - Generate lock files for flows
- \`wmill sync push\` - Deploy to Windmill

Use \`wmill resource-type list --schema\` to discover available resource types.

## Script Writing Guide

${SCRIPT_BASE}

You should use the write-script-<language> skill to write scripts in the language specified by the user.

## Flow Writing Guide

${FLOW_BASE}

You should use the write-flow skill to create flows.

## CLI Reference

${CLI_COMMANDS}

## Skills

For specific guidance, ALWAYS use the skills listed below.

${skillsReference}
`
      );
      log.info(colors.green("Created AGENTS.md"));
    }

    // Create CLAUDE.md file, referencing AGENTS.md
    if (!(await Deno.stat("CLAUDE.md").catch(() => null))) {
      await Deno.writeTextFile(
        "CLAUDE.md",
        `Instructions are in @AGENTS.md
`
      );
      log.info(colors.green("Created CLAUDE.md"));
    }

    // Create .claude/skills/ directory and skill files
    try {
      await Deno.mkdir(".claude/skills", { recursive: true });

      for (const skill of SKILLS) {
        const skillDir = `.claude/skills/${skill.name}`;
        await Deno.mkdir(skillDir, { recursive: true });

        const skillContent = SKILL_CONTENT[skill.name];
        if (skillContent) {
          await Deno.writeTextFile(`${skillDir}/SKILL.md`, skillContent);
        }
      }

      log.info(colors.green(`Created .claude/skills/ with ${SKILLS.length} skills`));
    } catch (skillError) {
      if (skillError instanceof Error) {
        log.warn(`Could not create skills: ${skillError.message}`);
      } else {
        log.warn(`Could not create skills: ${skillError}`);
      }
    }
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
