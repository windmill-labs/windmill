import { stat, writeFile, rm, mkdir } from "node:fs/promises";
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
import { SKILLS, SKILL_CONTENT, SCHEMAS, SCHEMA_MAPPINGS } from "../../guidance/skills.ts";
import { generateAgentsMdContent } from "../../guidance/core.ts";
import { generateCommentedTemplate } from "./template.ts";

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
          const selectedName = await Select.prompt({
            message: "Select workspace profile",
            options: profiles.map((p) => ({
              name: `${p.name} (${p.workspaceId} on ${p.remote})`,
              value: p.name,
            })),
            default: activeProfile?.name,
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
