import { colors, log, yamlStringify } from "../../../deps.ts";
import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { SyncOptions, readConfigFile } from "../../core/conf.ts";
import { deepEqual } from "../../utils/utils.ts";
import { getCurrentGitBranch, isGitRepository } from "../../utils/git.ts";

import { GitSyncRepository, WriteMode } from "./types.ts";
import { GitSyncSettingsConverter } from "./converter.ts";
import { handleLegacyRepositoryMigration } from "./legacySettings.ts";
import {
  selectAndLogRepository,
  generateStructuredDiff,
  generateChanges,
  displayChanges,
  getCurrentSettings,
  applyBackendSettingsToBranch,
  normalizeRepoPath,
  outputResult
} from "./utils.ts";

export async function pullGitSyncSettings(
  opts: GlobalOptions & {
    repository?: string;
    workspaceLevel?: boolean;
    default?: boolean;
    diff?: boolean;
    jsonOutput?: boolean;
    replace?: boolean;
    override?: boolean;
    withBackendSettings?: string;
    yes?: boolean;
  },
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  try {
    // Parse and validate --with-backend-settings if provided
    let settings: any;
    if (opts.withBackendSettings) {
      try {
        const parsedSettings = JSON.parse(opts.withBackendSettings);

        // Validate the structure matches expected format (raw settings object)
        if (
          !parsedSettings.include_path ||
          !Array.isArray(parsedSettings.include_path)
        ) {
          throw new Error(
            "Invalid settings format. Expected include_path array",
          );
        }
        if (
          !parsedSettings.include_type ||
          !Array.isArray(parsedSettings.include_type)
        ) {
          throw new Error(
            "Invalid settings format. Expected include_type array",
          );
        }

        // Create mock backend response with single repository using provided settings
        const mockRepositoryPath = opts.repository || "u/mock/repo";
        settings = {
          git_sync: {
            repositories: [{
              git_repo_resource_path: mockRepositoryPath,
              settings: {
                include_path: parsedSettings.include_path,
                include_type: parsedSettings.include_type,
                exclude_path: parsedSettings.exclude_path || [],
                extra_include_path: parsedSettings.extra_include_path || [],
              },
            }],
          },
        };
      } catch (parseError) {
        const errorMessage = parseError instanceof Error ? parseError.message : String(parseError);
        outputResult(opts, {
          success: false,
          error: `Failed to parse --with-backend-settings JSON: ${errorMessage}`,
        });
        return;
      }
    } else {
      // Fetch workspace settings to get git-sync configuration
      try {
        settings = await wmill.getSettings({
          workspace: workspace.workspaceId,
        });
      } catch (apiError) {
        const errorMessage = apiError instanceof Error ? apiError.message : String(apiError);
        outputResult(opts, {
          success: false,
          error: `Failed to fetch workspace settings: ${errorMessage}`,
        });
        return;
      }
    }

    if (
      !settings.git_sync?.repositories ||
      settings.git_sync.repositories.length === 0
    ) {
      outputResult(opts, {
        success: false,
        error: "No git-sync repositories configured",
      });
      return;
    }

    // Find the repository to work with
    let selectedRepo = await selectAndLogRepository(
      settings.git_sync.repositories,
      opts.repository,
    );

    // Check if the selected repository needs migration and handle it
    selectedRepo = await handleLegacyRepositoryMigration(
      selectedRepo,
      settings.git_sync,
      workspace,
      opts,
      "pull"
    );

    // Convert backend settings to SyncOptions format
    const backendSyncOptions: SyncOptions = GitSyncSettingsConverter.fromBackendFormat(selectedRepo.settings);

    // Check if wmill.yaml exists - require it for git-sync settings commands
    try {
      await Deno.stat("wmill.yaml");
    } catch (error) {
      log.error(
        colors.red(
          "No wmill.yaml file found. Please run 'wmill init' first to create the configuration file.",
        ),
      );
      Deno.exit(1);
    }

    // Read current local configuration
    const localConfig = await readConfigFile();

    // Calculate current settings once and reuse throughout
    const currentSettings = await getCurrentSettings(localConfig);

    // Determine write mode for branch-based configuration
    let writeMode: WriteMode = "replace";

    // Determine write mode based on flags
    if (opts.default || opts.replace) {
      writeMode = "replace";
    } else if (opts.override) {
      writeMode = "override";
    } else {
      // Default behavior for existing files with no explicit flags
      // Use same logic as diff to determine if there's a real conflict
      const gitSyncBackend = GitSyncSettingsConverter.extractGitSyncFields(
        GitSyncSettingsConverter.normalize(backendSyncOptions),
      );
      const gitSyncCurrent = GitSyncSettingsConverter.extractGitSyncFields(
        GitSyncSettingsConverter.normalize(currentSettings),
      );
      const hasConflict = !deepEqual(gitSyncBackend, gitSyncCurrent);

      if (hasConflict) {
        // For diff mode, show what override would look like
        writeMode = "override";
      } else {
        writeMode = "replace";
      }
    }

    if (opts.diff) {
      // Show differences between local and backend
      const normalizedCurrent = GitSyncSettingsConverter.normalize(currentSettings);
      const normalizedBackend = GitSyncSettingsConverter.normalize(backendSyncOptions);
      const gitSyncCurrent = GitSyncSettingsConverter.extractGitSyncFields(normalizedCurrent);
      const gitSyncBackend = GitSyncSettingsConverter.extractGitSyncFields(normalizedBackend);
      const hasChanges = !deepEqual(gitSyncBackend, gitSyncCurrent);

      if (opts.jsonOutput) {
        const repoPath = normalizeRepoPath(selectedRepo.git_repo_resource_path);

        // Generate structured diff using the same normalized objects
        const structuredDiff = hasChanges
          ? generateStructuredDiff(gitSyncCurrent, gitSyncBackend)
          : {};

        console.log(
          JSON.stringify({
            success: true,
            hasChanges,
            local: GitSyncSettingsConverter.toBackendFormat(normalizedCurrent),
            backend: GitSyncSettingsConverter.toBackendFormat(normalizedBackend),
            repository: selectedRepo.git_repo_resource_path,
            diff: structuredDiff,
          }),
        );
      } else {
        if (hasChanges) {
          log.info("Changes that would be applied locally:");
          const changes = generateChanges(currentSettings, backendSyncOptions);

          if (Object.keys(changes).length === 0) {
            log.info(colors.green("No differences found"));
          } else {
            displayChanges(changes);
          }
        } else {
          log.info(colors.green("No differences found"));
        }
      }
      return;
    }

    // For non-diff mode, handle interactive logic if needed
    // Only show interactive prompts for existing files with conflicts
    if (
      !opts.diff &&
      !opts.default &&
      !opts.replace &&
      !opts.override
    ) {
      // Use the same logic as diff to determine current settings
      const gitSyncBackend = GitSyncSettingsConverter.extractGitSyncFields(
        GitSyncSettingsConverter.normalize(backendSyncOptions),
      );
      const gitSyncCurrent = GitSyncSettingsConverter.extractGitSyncFields(
        GitSyncSettingsConverter.normalize(currentSettings),
      );
      const hasConflict = !deepEqual(gitSyncBackend, gitSyncCurrent);

      if (hasConflict && !opts.yes && Deno.stdin.isTerminal()) {
        // Show the diff first
        log.info("Changes that would be applied locally:");
        const changes = generateChanges(currentSettings, backendSyncOptions);
        if (Object.keys(changes).length > 0) {
          displayChanges(changes);
        }

        // Interactive mode - ask user
        const { Select } = await import("../../../deps.ts");
        const choice = await Select.prompt({
          message: "Settings conflict detected. How would you like to proceed?",
          options: [
            {
              name: "Replace existing settings",
              value: "replace",
            },
            {
              name: "Add branch-specific override",
              value: "override",
            },
            { name: "Cancel", value: "cancel" },
          ],
        });

        if (choice === "cancel") {
          log.info("Operation cancelled");
          return;
        }

        writeMode = choice as WriteMode;
      } else if (hasConflict && opts.yes) {
        // --yes flag: default to override behavior for conflicts
        writeMode = "override";
        log.info(colors.yellow("Settings conflict detected. Using --override behavior (default for --yes)."));
      } else if (hasConflict) {
        // Non-interactive mode with conflicts - show message and exit
        if (opts.jsonOutput) {
          console.log(
            JSON.stringify({
              success: false,
              error:
                "Settings conflict detected. Use --replace or --override flags to resolve.",
              hasConflict: true,
            }),
          );
        } else {
          log.error(colors.red("Settings conflict detected."));
          log.info(
            "Use --replace to overwrite existing settings or --override to add branch-specific override.",
          );
        }
        return;
      }
    }

    // Check if there are actually any changes before writing
    const gitSyncBackend = GitSyncSettingsConverter.extractGitSyncFields(
      GitSyncSettingsConverter.normalize(backendSyncOptions),
    );
    const gitSyncCurrent = GitSyncSettingsConverter.extractGitSyncFields(
      GitSyncSettingsConverter.normalize(currentSettings),
    );
    const hasActualChanges = !deepEqual(gitSyncBackend, gitSyncCurrent);

    if (!hasActualChanges) {
      // Even if no changes, check if we need to create empty branch structure
      let needsBranchStructure = false;
      if (isGitRepository()) {
        const currentBranch = getCurrentGitBranch();
        if (currentBranch && (!localConfig.git_branches || !localConfig.git_branches[currentBranch])) {
          needsBranchStructure = true;

          // Create empty branch structure
          const updatedConfig = { ...localConfig };
          if (!updatedConfig.git_branches) {
            updatedConfig.git_branches = {};
          }
          if (!updatedConfig.git_branches[currentBranch]) {
            updatedConfig.git_branches[currentBranch] = { overrides: {} };
          }

          // Write updated configuration
          await Deno.writeTextFile("wmill.yaml", yamlStringify(updatedConfig));

          if (opts.jsonOutput) {
            console.log(
              JSON.stringify({
                success: true,
                message: `Created empty branch structure for: ${currentBranch}`,
                repository: selectedRepo.git_repo_resource_path,
              }),
            );
          } else {
            log.info(
              colors.green(
                `Created empty branch structure for: ${currentBranch}`,
              ),
            );
          }
          return;
        }
      }

      if (opts.jsonOutput) {
        console.log(
          JSON.stringify({
            success: true,
            message: "No changes needed",
            repository: selectedRepo.git_repo_resource_path,
          }),
        );
      } else {
        log.info(
          colors.green(
            "No changes needed - settings are already up to date",
          ),
        );
      }
      return;
    }

    // Apply the settings based on write mode
    let updatedConfig: SyncOptions;


    if (writeMode === "replace") {
      // Replace top-level settings
      updatedConfig = { ...localConfig };
      // Update with backend git-sync settings
      Object.assign(updatedConfig, backendSyncOptions);
    } else {
      // Override mode - will be handled by branch logic below
      updatedConfig = { ...localConfig };
    }

    // For replace mode, add empty branch structure if in Git repo
    if (writeMode === "replace" && isGitRepository()) {
      const currentBranch = getCurrentGitBranch();
      if (currentBranch) {
        log.info(`Detected Git repository, adding empty branch structure for: ${currentBranch}`);
        if (!updatedConfig.git_branches) {
          updatedConfig.git_branches = {};
        }
        if (!updatedConfig.git_branches[currentBranch]) {
          updatedConfig.git_branches[currentBranch] = { overrides: {} };
        }
      }
    }

    // For override mode, write to branch overrides
    if (writeMode === "override" && isGitRepository()) {
      const currentBranch = getCurrentGitBranch();
      if (currentBranch) {
        log.info(`Detected Git repository, writing settings to branch: ${currentBranch}`);
        updatedConfig = applyBackendSettingsToBranch(localConfig, currentBranch, backendSyncOptions);
      }
    }

    // Write updated configuration
    await Deno.writeTextFile("wmill.yaml", yamlStringify(updatedConfig));

    if (opts.jsonOutput) {
      console.log(
        JSON.stringify({
          success: true,
          message: `Git-sync settings pulled successfully`,
          repository: selectedRepo.git_repo_resource_path,
        }),
      );
    } else {
      log.info(
        colors.green(
          `Git-sync settings pulled successfully from ${selectedRepo.git_repo_resource_path}`,
        ),
      );
      if (writeMode === "override") {
        const currentBranch = getCurrentGitBranch();
        if (currentBranch) {
          log.info(
            colors.gray(`Settings written to branch '${currentBranch}' overrides`),
          );
        }
      } else if (writeMode === "replace") {
        log.info(colors.gray(`Settings written to top-level configuration`));
      }
    }
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    if (opts.jsonOutput) {
      console.log(
        JSON.stringify({ success: false, error: errorMessage }),
      );
    } else {
      log.error(
        colors.red(
          `Failed to pull git-sync settings: ${errorMessage}`,
        ),
      );
    }
  }
}
