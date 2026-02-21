import { colors } from "@cliffy/ansi/colors";
import * as log from "@std/log";
import { deepEqual, selectRepository } from "../../utils/utils.ts";
import { SyncOptions, getEffectiveSettings, DEFAULT_SYNC_OPTIONS } from "../../core/conf.ts";
import { GitSyncRepository, GIT_SYNC_FIELDS } from "./types.ts";
import { GitSyncSettingsConverter } from "./converter.ts";

// Helper for consistent output handling between JSON and regular modes
export function outputResult(opts: { jsonOutput?: boolean }, result: {
  success: boolean;
  message?: string;
  error?: string;
  [key: string]: any;
}): void {
  if (opts.jsonOutput) {
    console.log(JSON.stringify(result));
  } else if (result.success && result.message) {
    log.info(colors.green(result.message));
  } else if (!result.success && result.error) {
    log.error(colors.red(result.error));
  }
}

// Helper to normalize repository path by removing $res: prefix
export function normalizeRepoPath(path: string): string {
  return path.replace(/^\$res:/, "");
}

// Helper to get or create branch configuration
export function getOrCreateBranchConfig(config: SyncOptions, branchName: string): {
  config: SyncOptions;
  branchKey: string;
} {
  if (!config.gitBranches) {
    config.gitBranches = {};
  }

  if (!config.gitBranches[branchName]) {
    config.gitBranches[branchName] = {};
  }

  return {
    config,
    branchKey: branchName
  };
}

// Helper to apply backend settings to branch configuration
export function applyBackendSettingsToBranch(
  config: SyncOptions,
  branchName: string,
  backendSettings: SyncOptions
): SyncOptions {
  const { config: updatedConfig } = getOrCreateBranchConfig(config, branchName);

  // Get the base settings (top-level + defaults) to compare against
  const { gitBranches, ...topLevelSettings } = config;
  const baseSettings: Partial<SyncOptions> = { ...DEFAULT_SYNC_OPTIONS, ...topLevelSettings };

  // Only store fields that differ from the base settings
  Object.keys(backendSettings).forEach(key => {
    if (key !== 'gitBranches' && backendSettings[key as keyof SyncOptions] !== undefined) {
      const backendValue = backendSettings[key as keyof SyncOptions];
      const baseValue = baseSettings[key as keyof SyncOptions];

      // Only store if different from base
      const isDifferent = GitSyncSettingsConverter.isDifferent(backendValue, baseValue);

      if (isDifferent) {
        if (!updatedConfig.gitBranches![branchName].overrides) {
          updatedConfig.gitBranches![branchName].overrides = {};
        }
        (updatedConfig.gitBranches![branchName].overrides as any)[key] = backendValue;
      }
    }
  });

  return updatedConfig;
}

// Select repository interactively if multiple exist
export async function selectAndLogRepository(
  repositories: GitSyncRepository[],
  repository?: string,
  suppressLogs?: boolean,
): Promise<GitSyncRepository> {
  let selectedRepo: GitSyncRepository;

  if (repository) {
    const found = repositories.find(
      (r: GitSyncRepository) =>
        r.git_repo_resource_path === repository ||
        r.git_repo_resource_path === `$res:${repository}`,
    );
    if (!found) {
      throw new Error(`Repository ${repository} not found`);
    }
    selectedRepo = found;
    const repoPath = selectedRepo.git_repo_resource_path.replace(/^\$res:/, "");
    if (!suppressLogs) {
      log.info(colors.cyan(`Using repository: ${colors.bold(repoPath)}`));
    }
  } else {
    selectedRepo = await selectRepository(repositories);
  }

  return selectedRepo;
}

// Generate structured diff showing field changes
export function generateStructuredDiff(
  current: any,
  backend: any,
): { [key: string]: { from: any; to: any } } {
  const diff: { [key: string]: { from: any; to: any } } = {};

  // Get all unique keys from both objects
  const allKeys = new Set([...Object.keys(current), ...Object.keys(backend)]);

  for (const key of allKeys) {
    const currentValue = current[key];
    const backendValue = backend[key];

    if (!deepEqual(currentValue, backendValue)) {
      diff[key] = {
        from: currentValue,
        to: backendValue,
      };
    }
  }

  return diff;
}

// Helper to generate changes between two SyncOptions objects (normalizes automatically)
export function generateChanges(
  current: SyncOptions,
  new_: SyncOptions,
): { [key: string]: { from: any; to: any } } {
  const changes: { [key: string]: { from: any; to: any } } = {};

  // Normalize both inputs for consistent comparison
  const normalizedCurrent = GitSyncSettingsConverter.normalize(current);
  const normalizedNew = GitSyncSettingsConverter.normalize(new_);

  for (const field of GIT_SYNC_FIELDS) {
    const currentValue = (normalizedCurrent as any)[field];
    const newValue = (normalizedNew as any)[field];
    if (!deepEqual(currentValue, newValue)) {
      changes[field] = {
        from: currentValue,
        to: newValue,
      };
    }
  }

  return changes;
}

// Helper to display changes in human-readable format
export function displayChanges(
  changes: { [key: string]: { from: any; to: any } },
): void {
  for (const [field, change] of Object.entries(changes)) {
    if (
      Array.isArray(change.from) ||
      Array.isArray(change.to)
    ) {
      console.log(colors.yellow(`  ${field}:`));
      console.log(
        colors.red(
          `    - ${JSON.stringify(change.from)}`,
        ),
      );
      console.log(
        colors.green(
          `    + ${JSON.stringify(change.to)}`,
        ),
      );
    } else {
      console.log(
        colors.yellow(`  ${field}: `) +
          colors.red(`${change.from}`) +
          " → " +
          colors.green(`${change.to}`),
      );
    }
  }
}
