import { stat, writeFile, rm, mkdir, readdir } from "node:fs/promises";
import { join } from "node:path";
import { writeFileSync, mkdirSync } from "node:fs";
import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import { Confirm } from "@cliffy/prompt/confirm";
import * as log from "../../core/log.ts";
import { stringify as yamlStringify } from "yaml";
import { GlobalOptions } from "../../types.ts";
import { readLockfile } from "../../utils/metadata.ts";
import { getActiveWorkspaceOrFallback } from "../workspace/workspace.ts";
import { generateRTNamespace } from "../resource-type/resource-type.ts";
import { SKILLS, SKILL_CONTENT, SCHEMAS, SCHEMA_MAPPINGS } from "../../guidance/skills.ts";
import { generateAgentsMdContent } from "../../guidance/core.ts";

/**
 * Format a YAML schema for inclusion in skill markdown files.
 */
function formatSchemaForMarkdown(schemaYaml: string, schemaName: string, filePattern: string): string {
  return `## ${schemaName} (\`${filePattern}\`)

Must be a YAML file that adheres to the following schema:

\`\`\`yaml
${schemaYaml.trim()}
\`\`\``;
}

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
  if (await stat("wmill.yaml").catch(() => null)) {
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
    await writeFile("wmill.yaml", yamlStringify(initialConfig), "utf-8");
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
          !!process.stdin.isTTY &&
          !opts.useDefault;

        const shouldSkip =
          opts.bindProfile != true &&
          (opts.useDefault || !!!process.stdin.isTTY);

        if (!shouldSkip) {
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

            await writeFile("wmill.yaml", yamlStringify(currentConfig), "utf-8");

            log.info(
              colors.green(
                `✓ Bound branch '${currentBranch}' to workspace '${activeWorkspace.name}'`
              )
            );
          }
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
    // Generate skills reference section for AGENTS.md
    const skills_base_dir = ".claude/skills";
    const skillsReference = SKILLS.map(
      (s) => `- \`${skills_base_dir}/${s.name}/SKILL.md\` - ${s.description}`
    ).join("\n");

    // Create AGENTS.md file with minimal instructions
    if (!(await stat("AGENTS.md").catch(() => null))) {
      await writeFile(
        "AGENTS.md",
        generateAgentsMdContent(skillsReference), "utf-8"
      );
      log.info(colors.green("Created AGENTS.md"));
    }

    // Create CLAUDE.md file, referencing AGENTS.md
    if (!(await stat("CLAUDE.md").catch(() => null))) {
      await writeFile(
        "CLAUDE.md",
        `Instructions are in @AGENTS.md
`, "utf-8"
      );
      log.info(colors.green("Created CLAUDE.md"));
    }

    // Create .claude/skills/ directory and skill files
    try {
      await mkdir(".claude/skills", { recursive: true });

      await Promise.all(
        SKILLS.map(async (skill) => {
          const skillDir = `.claude/skills/${skill.name}`;
          await mkdir(skillDir, { recursive: true });

          let skillContent = SKILL_CONTENT[skill.name];
          if (skillContent) {
            // Replace placeholders with actual suffixes based on nonDottedPaths
            if (nonDottedPaths) {
              skillContent = skillContent
                .replaceAll("{{FLOW_SUFFIX}}", "__flow")
                .replaceAll("{{APP_SUFFIX}}", "__app")
                .replaceAll("{{RAW_APP_SUFFIX}}", "__raw_app")
                .replaceAll("{{INLINE_SCRIPT_NAMING}}", "Inline script files should NOT include `.inline_script.` in their names (e.g. use `a.ts`, not `a.inline_script.ts`).");
            } else {
              skillContent = skillContent
                .replaceAll("{{FLOW_SUFFIX}}", ".flow")
                .replaceAll("{{APP_SUFFIX}}", ".app")
                .replaceAll("{{RAW_APP_SUFFIX}}", ".raw_app")
                .replaceAll("{{INLINE_SCRIPT_NAMING}}", "Inline script files use the `.inline_script.` naming convention (e.g. `a.inline_script.ts`).");
            }
            // Check if this skill has schemas that need to be appended
            const schemaMappings = SCHEMA_MAPPINGS[skill.name];
            if (schemaMappings && schemaMappings.length > 0) {
              // Combine base content with schemas
              const schemaDocs = schemaMappings
                .map((mapping) => {
                  const schemaYaml = SCHEMAS[mapping.schemaKey];
                  if (schemaYaml) {
                    return formatSchemaForMarkdown(schemaYaml, mapping.name, mapping.filePattern);
                  }
                  return null;
                })
                .filter((doc): doc is string => doc !== null);

              if (schemaDocs.length > 0) {
                skillContent = skillContent + "\n\n" + schemaDocs.join("\n\n");
              }
            }

            await writeFile(`${skillDir}/SKILL.md`, skillContent, "utf-8");
          }
        })
      );

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

  // Generate .claude/launch.json for each flow folder
  try {
    const flowSuffix = nonDottedPaths ? "__flow" : ".flow";
    const flowLaunchJson = JSON.stringify({
      version: "0.0.1",
      configurations: [{
        name: "windmill",
        runtimeExecutable: "bash",
        runtimeArgs: ["-c", "wmill dev --proxy-port ${PORT:-4000}"],
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
