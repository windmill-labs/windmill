import process from "node:process";

import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { Confirm } from "@cliffy/prompt/confirm";
import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { SyncOptions, readConfigFile, validateBranchConfiguration, getEffectiveSettings, getWmillYamlPath } from "../../core/conf.ts";
import { deepEqual } from "../../utils/utils.ts";

import { GitSyncRepository } from "./types.ts";
import { GitSyncSettingsConverter } from "./converter.ts";
import { handleLegacyRepositoryMigration } from "./legacySettings.ts";
import {
  selectAndLogRepository,
  generateStructuredDiff,
  generateChanges,
  displayChanges,
  normalizeRepoPath,
  outputResult
} from "./utils.ts";

export async function pushGitSyncSettings(
  opts: GlobalOptions & {
    repository?: string;
    diff?: boolean;
    jsonOutput?: boolean;
    withBackendSettings?: string;
    yes?: boolean;
    promotion?: string;
  },
) {
  // Validate branch configuration like sync commands
  try {
    await validateBranchConfiguration({ yes: opts.yes });
  } catch (error) {
    if (error instanceof Error && error.message.includes("overrides")) {
      log.error(error.message);
      process.exit(1);
    }
    throw error;
  }

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  try {
    // Check if wmill.yaml exists - require it for git-sync settings commands
    const wmillYamlPath = getWmillYamlPath();
    if (!wmillYamlPath) {
      log.error(
        colors.red(
          "No wmill.yaml file found. Please run 'wmill init' first to create the configuration file.",
        ),
      );
      process.exit(1);
    }

    // Read local configuration
    const localConfig = await readConfigFile();

    // Parse and validate --with-backend-settings if provided, otherwise fetch from backend
    let settings: any;
    if (opts.withBackendSettings) {
      try {
        const parsedSettings = JSON.parse(opts.withBackendSettings);

        // Validate the structure matches expected test format (raw settings object)
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
      // Fetch current backend settings
      try {
        settings = await wmill.getSettings({
          workspace: workspace.workspaceId,
        });
      } catch (apiError) {
        const errorMessage = apiError instanceof Error ? apiError.message : String(apiError);
        if (opts.jsonOutput) {
          console.log(
            JSON.stringify({
              success: false,
              error: `Failed to fetch workspace settings: ${errorMessage}`,
            }),
          );
        } else {
          log.error(
            colors.red(
              `Failed to fetch workspace settings: ${errorMessage}`,
            ),
          );
        }
        return;
      }
    }

    if (
      !settings.git_sync?.repositories ||
      settings.git_sync.repositories.length === 0
    ) {
      if (opts.jsonOutput) {
        console.log(
          JSON.stringify({
            success: false,
            error: "No git-sync repositories configured",
          }),
        );
      } else {
        log.error(
          colors.red(
            "No git-sync repositories configured in workspace",
          ),
        );
      }
      return;
    }

    // Find the repository to work with
    let selectedRepo = await selectAndLogRepository(
      settings.git_sync.repositories,
      opts.repository,
      opts.jsonOutput,
    );

    // Check if the selected repository needs migration and handle it
    selectedRepo = await handleLegacyRepositoryMigration(
      selectedRepo,
      settings.git_sync,
      workspace,
      opts,
      "push"
    );

    // If migration was cancelled, exit
    if (selectedRepo === null) {
      return;
    }

    // Get effective settings for this workspace/repo
    const repoPath = normalizeRepoPath(selectedRepo.git_repo_resource_path);
    const effectiveSettings = await getEffectiveSettings(localConfig, opts.promotion, true, opts.jsonOutput);

    // Convert to backend format
    const backendFormat = GitSyncSettingsConverter.toBackendFormat(effectiveSettings);

    // Calculate diff for all modes
    const currentBackend = selectedRepo.settings;

    // Convert current backend settings to SyncOptions for user-friendly display
    const currentSyncOptions: SyncOptions = GitSyncSettingsConverter.fromBackendFormat(currentBackend);

    const normalizedCurrent = GitSyncSettingsConverter.normalize(currentSyncOptions);
    const normalizedEffective = GitSyncSettingsConverter.normalize(effectiveSettings);
    const gitSyncCurrent = GitSyncSettingsConverter.extractGitSyncFields(normalizedCurrent);
    const gitSyncEffective = GitSyncSettingsConverter.extractGitSyncFields(normalizedEffective);
    const hasChanges = !deepEqual(gitSyncEffective, gitSyncCurrent);

    if (opts.diff) {
      // --diff flag: show differences and exit
      if (opts.jsonOutput) {
        // Generate structured diff using the same normalized objects
        const structuredDiff = hasChanges
          ? generateStructuredDiff(gitSyncCurrent, gitSyncEffective)
          : {};

        console.log(
          JSON.stringify({
            success: true,
            hasChanges,
            local: GitSyncSettingsConverter.toBackendFormat(normalizedEffective),
            backend: GitSyncSettingsConverter.toBackendFormat(normalizedCurrent),
            repository: selectedRepo.git_repo_resource_path,
            diff: structuredDiff,
          }),
        );
      } else {
        if (hasChanges) {
          log.info("Changes that would be pushed to Windmill:");
          const changes = generateChanges(
            currentSyncOptions,
            effectiveSettings,
          );

          if (Object.keys(changes).length === 0) {
            log.info(colors.green("No changes to push"));
          } else {
            displayChanges(changes);
          }
        } else {
          log.info(colors.green("No changes to push"));
        }
      }
      return;
    }

    // Default behavior: show changes and ask for confirmation (unless --yes is passed)
    if (hasChanges) {
      if (!opts.jsonOutput) {
        const changes = generateChanges(
          currentSyncOptions,
          effectiveSettings,
        );

        if (Object.keys(changes).length === 0) {
          log.info(colors.green("No changes to push"));
          return;
        } else {
          log.info("Changes that would be pushed to Windmill:");
          displayChanges(changes);
        }
      }

      // Ask for confirmation unless --yes is passed or not in TTY
      if (!opts.yes && !!process.stdin.isTTY) {
        const confirmed = await Confirm.prompt({
          message: `Do you want to apply these changes to the remote?`,
          default: true,
        });

        if (!confirmed) {
          return;
        }
      }
    } else {
      log.info(colors.green("No changes to push"));
      return;
    }

    if (opts.withBackendSettings) {
      // Skip backend update when using simulated settings
      if (opts.jsonOutput) {
        console.log(
          JSON.stringify({
            success: true,
            message:
              `Git-sync settings push simulated (--with-backend-settings used)`,
            repository: selectedRepo.git_repo_resource_path,
            simulated: true,
          }),
        );
      } else {
        log.info(
          colors.green(
            `Git-sync settings push simulated for ${selectedRepo.git_repo_resource_path} (--with-backend-settings used)`,
          ),
        );
      }
    } else {
      // Update the specific repository settings
      const updatedRepos = settings.git_sync.repositories.map(
        (repo: GitSyncRepository) => {
          if (
            repo.git_repo_resource_path ===
              selectedRepo.git_repo_resource_path
          ) {
            return {
              ...repo,
              settings: backendFormat,
            };
          }
          return repo;
        },
      );

      // Push updated settings to backend
      try {
        await wmill.editWorkspaceGitSyncConfig({
          workspace: workspace.workspaceId,
          requestBody: {
            git_sync_settings: {
              repositories: updatedRepos,
            },
          },
        });
      } catch (apiError) {
        const errorMessage = apiError instanceof Error ? apiError.message : String(apiError);
        if (opts.jsonOutput) {
          console.log(
            JSON.stringify({
              success: false,
              error: `Failed to update workspace git-sync config: ${errorMessage}`,
            }),
          );
        } else {
          log.error(
            colors.red(
              `Failed to update workspace git-sync config: ${errorMessage}`,
            ),
          );
        }
        return;
      }

      if (opts.jsonOutput) {
        console.log(
          JSON.stringify({
            success: true,
            message: `Git-sync settings pushed successfully`,
            repository: selectedRepo.git_repo_resource_path,
          }),
        );
      } else {
        log.info(
          colors.green(
            `Git-sync settings pushed successfully to ${selectedRepo.git_repo_resource_path}`,
          ),
        );
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
          `Failed to push git-sync settings: ${errorMessage}`,
        ),
      );
    }
  }
}
