import type { SyncOptions, WorkspaceProfile } from "./conf.ts";
import { DEFAULT_SYNC_OPTIONS } from "./conf.ts";

// Interface for structured responses
export interface UIState {
  include_path: string[];
  include_type: string[];
}

// ========== REPOSITORY PATH UTILITIES ==========

// Helper function to normalize repository path (add $res: prefix if not present)
export function normalizeRepositoryPath(repositoryPath: string): string {
  return repositoryPath.startsWith('$res:') ? repositoryPath : `$res:${repositoryPath}`;
}

// Helper function to display repository path (remove $res: prefix for user display)
export function displayRepositoryPath(repositoryPath: string): string {
  return repositoryPath.startsWith('$res:') ? repositoryPath.substring(5) : repositoryPath;
}

// ========== INTERACTIVE REPOSITORY SELECTION ==========

export async function selectRepositoryInteractively(
  availableRepositories: Array<{ git_repo_resource_path: string; display_path: string }>,
  operation: string
): Promise<string> {
  if (availableRepositories.length === 0) {
    throw new Error("No repositories found in workspace");
  }

  if (availableRepositories.length === 1) {
    return availableRepositories[0].display_path;
  }

  // Import Select dynamically to avoid dependency issues
  const { Select } = await import("./deps.ts");

  console.log(`\nMultiple repositories found. Please select which repository to ${operation}:\n`);

  const selectedRepo = await Select.prompt({
    message: `Select repository for ${operation}:`,
    options: availableRepositories.map((repo, index) => ({
      name: `${index + 1}. ${repo.display_path}`,
      value: repo.display_path
    }))
  });

  return selectedRepo;
}

// ========== SYNC OPTIONS UTILITIES ==========

export function uiStateToSyncOptions(uiState: UIState): SyncOptions {
  return {
    ...DEFAULT_SYNC_OPTIONS,
    includes: uiState.include_path.length > 0 ? uiState.include_path : DEFAULT_SYNC_OPTIONS.includes,

    // Convert include_type array to skip/include flags
    skipVariables: !uiState.include_type.includes('variable'),
    skipResources: !uiState.include_type.includes('resource'),
    skipResourceTypes: !uiState.include_type.includes('resourcetype'),
    skipSecrets: !uiState.include_type.includes('secret'),

    includeSchedules: uiState.include_type.includes('schedule'),
    includeTriggers: uiState.include_type.includes('trigger'),
    includeUsers: uiState.include_type.includes('user'),
    includeGroups: uiState.include_type.includes('group'),
    includeSettings: uiState.include_type.includes('settings'),
    includeKey: uiState.include_type.includes('key')
  };
}

export function syncOptionsToUIState(syncOptions: SyncOptions): UIState {
  const include_type: string[] = ['script', 'flow', 'app', 'folder']; // Always included

  // Add types based on skip flags (default to included if not specified)
  if (syncOptions.skipVariables !== true) include_type.push('variable');
  if (syncOptions.skipResources !== true) include_type.push('resource');
  if (syncOptions.skipResourceTypes !== true) include_type.push('resourcetype');
  if (syncOptions.skipSecrets !== true) include_type.push('secret');

  // Add types based on include flags (default to excluded if not specified)
  if (syncOptions.includeSchedules === true) include_type.push('schedule');
  if (syncOptions.includeTriggers === true) include_type.push('trigger');
  if (syncOptions.includeUsers === true) include_type.push('user');
  if (syncOptions.includeGroups === true) include_type.push('group');
  if (syncOptions.includeSettings === true) include_type.push('settings');
  if (syncOptions.includeKey === true) include_type.push('key');

  return {
    include_path: syncOptions.includes || DEFAULT_SYNC_OPTIONS.includes,
    include_type
  };
}

// Helper function to handle JSON input parsing
export function parseJsonInput(jsonInput: string): UIState {
  try {
    return JSON.parse(jsonInput);
  } catch (e) {
    throw new Error("Invalid JSON in settings parameter: " + (e as Error).message);
  }
}

// ========== REPOSITORY RESOLUTION UTILITIES ==========

/**
 * Helper function to resolve workspace and repository path from config, interactive selection, or direct specification
 */
export async function resolveWorkspaceAndRepository(
  specifiedWorkspace?: string,
  specifiedRepository?: string
): Promise<{ workspace?: string; repository?: string }> {
  try {
    const { mergeConfigWithConfigFile, listWorkspaces, listWorkspaceRepositories } = await import("./conf.ts");
    const localConfig = await mergeConfigWithConfigFile({});

    // Handle workspace resolution
    let resolvedWorkspace = specifiedWorkspace;
    if (!resolvedWorkspace) {
      if (localConfig.defaultWorkspace) {
        resolvedWorkspace = localConfig.defaultWorkspace;
      } else if (localConfig.workspaces) {
        const workspaceNames = listWorkspaces(localConfig);
        if (workspaceNames.length === 1) {
          resolvedWorkspace = workspaceNames[0];
        } else if (workspaceNames.length > 1) {
          // Multiple workspaces - would need interactive selection
          // For now, throw error requiring explicit specification
          throw new Error(`Multiple workspaces configured: ${workspaceNames.join(', ')}. Please specify --workspace.`);
        }
      }
    }

    // Handle repository resolution
    let resolvedRepository = specifiedRepository;
    if (!resolvedRepository && resolvedWorkspace) {
      const workspaceProfile = localConfig.workspaces?.[resolvedWorkspace];
      if (workspaceProfile?.currentRepository) {
        resolvedRepository = workspaceProfile.currentRepository;
      } else {
        const repoNames = listWorkspaceRepositories(localConfig, resolvedWorkspace);
        if (repoNames.length === 1) {
          resolvedRepository = repoNames[0];
        } else if (repoNames.length > 1) {
          // Multiple repositories in workspace - show interactive selection
          const repoOptions = repoNames.map(repo => ({
            git_repo_resource_path: normalizeRepositoryPath(repo),
            display_path: repo
          }));
          resolvedRepository = await selectRepositoryInteractively(repoOptions, "work with");
        }
      }
    }

    // Fallback to legacy repository resolution if no workspace config
    if (!resolvedRepository && !resolvedWorkspace) {
      resolvedRepository = resolveRepositoryPathLegacy(specifiedRepository);
    }

    return {
      workspace: resolvedWorkspace,
      repository: resolvedRepository
    };
  } catch (error) {
    throw error;
  }
}

/**
 * Legacy repository resolution (for backward compatibility)
 */
export function resolveRepositoryPathLegacy(specifiedRepository?: string): string | undefined {
  return specifiedRepository || undefined;
}



/**
 * Helper function to resolve workspace and repository for sync operations
 * Returns workspace profile, repository path, and applicable sync options
 */
export async function resolveWorkspaceAndRepositoryForSync(
  specifiedWorkspace?: string,
  specifiedRepository?: string
): Promise<{
  workspaceName?: string;
  workspaceProfile?: WorkspaceProfile;
  repositoryPath?: string;
  syncOptions: SyncOptions;
}> {
  try {
    const { mergeConfigWithConfigFile, getWorkspaceProfile, getWorkspaceRepositorySettings } = await import("./conf.ts");
    const baseConfig = await mergeConfigWithConfigFile({});

    // Resolve workspace and repository
    const { workspace: workspaceName, repository: repositoryPath } = await resolveWorkspaceAndRepository(
      specifiedWorkspace,
      specifiedRepository
    );

    let workspaceProfile: WorkspaceProfile | undefined = undefined;
    let syncOptions: SyncOptions;

    if (workspaceName) {
      // Get workspace profile
      workspaceProfile = getWorkspaceProfile(baseConfig, workspaceName);

      if (repositoryPath) {
        // Get workspace + repository specific settings
        syncOptions = getWorkspaceRepositorySettings(baseConfig, workspaceName, repositoryPath);
      } else {
        // Get workspace-level settings
        syncOptions = getWorkspaceRepositorySettings(baseConfig, workspaceName, '');
      }
    } else {
      // Legacy format: just use base config (no repository awareness)
      syncOptions = baseConfig;
    }

    return {
      workspaceName,
      workspaceProfile,
      repositoryPath,
      syncOptions
    };
  } catch (error) {
    // Fall back to legacy mode for backward compatibility
    const { mergeConfigWithConfigFile } = await import("./conf.ts");
    try {
      const legacyConfig = await mergeConfigWithConfigFile({});
      return {
        workspaceName: undefined,
        workspaceProfile: undefined,
        repositoryPath: specifiedRepository,
        syncOptions: legacyConfig
      };
    } catch {
      return {
        workspaceName: undefined,
        workspaceProfile: undefined,
        repositoryPath: specifiedRepository,
        syncOptions: {}
      };
    }
  }
}

