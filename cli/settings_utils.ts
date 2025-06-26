import type { SyncOptions, WorkspaceProfile } from "./conf.ts";
import { DEFAULT_SYNC_OPTIONS } from "./conf.ts";
import { stringify } from "jsr:@std/yaml@^1.0.5";

// Interface for structured responses
export interface UIState {
  include_path: string[];
  include_type: string[];
  exclude_path?: string[];
  extra_include_path?: string[];
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

  // Check if we're in a non-interactive environment (e.g., tests, CI/CD)
  const isInteractive = Deno.stdin.isTerminal() && Deno.stdout.isTerminal();
  
  if (!isInteractive) {
    // In non-interactive environments, require explicit repository specification
    throw new Error(`Multiple repositories found: ${availableRepositories.map(r => r.display_path).join(', ')}. Use --repository to specify which one to ${operation}.`);
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
  // Provide defaults for potentially missing fields to handle permissive JSON input
  const includeTypes = uiState.include_type || [];
  
  return {
    ...DEFAULT_SYNC_OPTIONS,
    includes: uiState.include_path || [],
    excludes: uiState.exclude_path || [],
    extraIncludes: uiState.extra_include_path || [],

    // Core types
    skipScripts: !includeTypes.includes('script'),
    skipFlows: !includeTypes.includes('flow'),
    skipApps: !includeTypes.includes('app'),
    skipFolders: !includeTypes.includes('folder'),

    // Secondary types
    skipVariables: !includeTypes.includes('variable'),
    skipResources: !includeTypes.includes('resource'),
    skipResourceTypes: !includeTypes.includes('resourcetype'),
    skipSecrets: !includeTypes.includes('secret'),

    includeSchedules: includeTypes.includes('schedule'),
    includeTriggers: includeTypes.includes('trigger'),
    includeUsers: includeTypes.includes('user'),
    includeGroups: includeTypes.includes('group'),
    includeSettings: includeTypes.includes('settings'),
    includeKey: includeTypes.includes('key')
  };
}

export function syncOptionsToUIState(syncOptions: SyncOptions): UIState {
  const include_type: string[] = [];

  // Core types (default to included unless explicitly skipped)
  if (syncOptions.skipScripts !== true) include_type.push('script');
  if (syncOptions.skipFlows !== true) include_type.push('flow');
  if (syncOptions.skipApps !== true) include_type.push('app');
  if (syncOptions.skipFolders !== true) include_type.push('folder');

  // Secondary types based on skip flags (default to included if not specified)
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
    include_path: syncOptions.includes || [],
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
  specifiedRepository?: string,
  providedConfig?: SyncOptions
): Promise<{ workspace?: string; repository?: string }> {
  try {
    const { listWorkspaces, listWorkspaceRepositories } = await import("./conf.ts");
    const localConfig = providedConfig || {};

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
  specifiedRepository?: string,
  providedConfig?: SyncOptions
): Promise<{
  workspaceName?: string;
  workspaceProfile?: WorkspaceProfile;
  repositoryPath?: string;
  syncOptions: SyncOptions;
}> {
  try {
    const { getWorkspaceProfile, getWorkspaceRepositorySettings, readConfigFile } = await import("./conf.ts");
    const baseConfig = providedConfig || await readConfigFile();

    // Resolve workspace and repository
    const { workspace: workspaceName, repository: repositoryPath } = await resolveWorkspaceAndRepository(
      specifiedWorkspace,
      specifiedRepository,
      baseConfig
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

    // If we have empty sync options (no local config), mark for backend fetch
    if (Object.keys(syncOptions).length === 0 && workspaceName) {
      syncOptions = { __needsBackendFetch: true } as any;
    }

    // If we don't have either workspace or repository from config, throw to trigger fallback
    if (!workspaceName && !repositoryPath) {
      throw new Error("No workspace or repository found in multi-workspace config");
    }

    return {
      workspaceName,
      workspaceProfile,
      repositoryPath,
      syncOptions
    };
  } catch (error) {
    // Fall back to legacy mode for backward compatibility
    try {
      const legacyConfig = providedConfig || {};
      if (!specifiedWorkspace && !specifiedRepository) {
        throw new Error("No workspace or repository specified and no multi-workspace config found");
      }

      return {
        workspaceName: undefined,
        workspaceProfile: undefined,
        repositoryPath: specifiedRepository,
        syncOptions: legacyConfig
      };
    } catch (innerError) {
      if (!specifiedWorkspace && !specifiedRepository) {
        throw new Error("No workspace or repository specified and config file error");
      }

      return {
        workspaceName: undefined,
        workspaceProfile: undefined,
        repositoryPath: specifiedRepository,
        syncOptions: { __needsBackendFetch: true } as any  // Special marker to indicate backend fetch needed
      };
    }
  }
}

// ========== NORMALISATION HELPERS ==========

/**
 * Base utility for cleaning objects with configurable options
 */
function cleanObject(
  obj: Record<string, unknown>, 
  options: {
    removeUndefined?: boolean;
    removeFalse?: boolean; 
    removeEmptyArrays?: boolean;
    removeLocalFields?: boolean;
    clone?: boolean;
  } = {}
): Record<string, unknown> {
  const { 
    removeUndefined = true, 
    removeFalse = true, 
    removeEmptyArrays = true, 
    removeLocalFields = false,
    clone = false
  } = options;
  
  let cleaned = clone ? JSON.parse(JSON.stringify(obj)) : obj;
  
  if (removeLocalFields) {
    const { codebases, defaultTs, ...rest } = cleaned as any;
    cleaned = rest;
  }

  const result = { ...cleaned };
  Object.entries(result).forEach(([k, v]) => {
    if ((removeUndefined && v === undefined) ||
        (removeFalse && v === false) ||
        (removeEmptyArrays && Array.isArray(v) && v.length === 0)) {
      delete result[k];
    }
  });
  return result;
}

/**
 * Remove fields that do not affect semantics when comparing settings.
 * - Drops undefined and boolean false values
 * - Drops empty arrays
 * - Drops `codebases` and `defaultTs` completely (local-only fields)
 */
export function createSettingsForComparison<T extends SyncOptions | Record<string, unknown>>(settings: T): any {
  return cleanObject(settings as Record<string, unknown>, { removeLocalFields: true });
}

// Produce a comparison object that keeps explicit `false` or empty-array values when they
// represent a change between reference and candidate (used for diff views).
export function createSettingsForDiff(reference: SyncOptions, candidate: SyncOptions): SyncOptions {
  const cleaned = createSettingsForComparison(candidate) as any;

  for (const [key, refVal] of Object.entries(reference as any)) {
    if (!(key in cleaned)) {
      const candVal = (candidate as any)[key];
      const changedToFalse = typeof refVal === "boolean" && candVal === false;
      const changedToEmptyArr = Array.isArray(refVal) && Array.isArray(candVal) && candVal.length === 0;
      if (changedToFalse || changedToEmptyArr) {
        cleaned[key] = candVal;
      }
    }
  }
  return cleaned;
}

/**
 * Prepare settings object for YAML serialisation so default/empty values are not
 * considered differences during diffing operations.
 */
export function sanitizeSyncOptionsForYaml(o: SyncOptions | Record<string, unknown>): Record<string, unknown> {
  return cleanObject(o as Record<string, unknown>, { clone: true });
}

/**
 * Merge target object with redundant keys (false / empty array) that are present
 * in existing, ensuring we never drop explicit declarations the user already had.
 * This runs AFTER `sanitizeSyncOptionsForYaml` removed such keys.
 */
export function mergePreserveRedundantKeys<T extends Record<string, unknown>>(existing: any, target: T): T {
  if (existing == null || typeof existing !== "object") return target;

  for (const [key, value] of Object.entries(existing)) {
    if (!(key in target)) {
      // key was omitted by sanitizer; keep it if user had it explicitly
      (target as any)[key] = value;
    } else if (
      value !== null &&
      typeof value === "object" &&
      !Array.isArray(value) &&
      typeof (target as any)[key] === "object" &&
      !Array.isArray((target as any)[key])
    ) {
      // recurse into nested objects (e.g. workspace -> repositories)
      mergePreserveRedundantKeys(value as any, (target as any)[key]);
    }
  }
  return target;
}

// ---------- Generic order-preserving merge helper ----------

export function mergePreservingOrder<T extends Record<string, unknown>>(existing: Record<string, unknown>, target: T, keepFalsy = true): T {
  const result: Record<string, unknown> = {};
  const processed = new Set<string>();

  for (const key of Object.keys(existing)) {
    if (key in target) {
      result[key] = (target as any)[key];
      processed.add(key);
    } else {
      result[key] = existing[key];
    }
  }

  for (const key of Object.keys(target)) {
    if (processed.has(key)) continue;
    const v = (target as any)[key];
    const redundant = v === undefined || (typeof v === "boolean" && v === false) || (Array.isArray(v) && v.length === 0);
    if (keepFalsy || !redundant) {
      result[key] = v;
    }
  }
  return result as T;
}

// ---------- YAML diff helper ----------
import { createTwoFilesPatch } from "npm:diff";
export async function yamlDiff(objA: unknown, objB: unknown): Promise<string> {
  const a = yamlSafe(objA);
  const b = yamlSafe(objB);
  if (a === b) return "";
  return createTwoFilesPatch("a.yaml", "b.yaml", a, b, "", "", { context: 3 });
}

// ---------- Equality helper ----------
export function syncOptionsEqual(a: SyncOptions, b: SyncOptions): boolean {
  return JSON.stringify(createSettingsForComparison(a)) === JSON.stringify(createSettingsForComparison(b));
}

// ================= YAML & SyncOption helpers (shared) =================

// Safe YAML stringify that first removes undefined / function fields.
export function yamlSafe(value: unknown): string {
  const cleaned = JSON.parse(JSON.stringify(value));
  return stringify(cleaned as Record<string, unknown>);
}

// YAML stringify with consistent field ordering for semantic comparisons
export function yamlSafeForComparison(value: unknown): string {
  const cleaned = JSON.parse(JSON.stringify(value));

  // Then sort keys for consistent ordering
  if (cleaned && typeof cleaned === 'object' && !Array.isArray(cleaned)) {
    const sortedObj = Object.keys(cleaned).sort().reduce((acc, key) => {
      acc[key] = cleaned[key];
      return acc;
    }, {} as Record<string, unknown>);
    return stringify(sortedObj);
  }
  return stringify(cleaned as Record<string, unknown>);
}

// Extract only the fields that are relevant to backend syncing (drops codebases and defaultTs).
export function extractSyncOptions(cfg: SyncOptions): SyncOptions {
  return {
    includes: cfg.includes,
    extraIncludes: (cfg as any).extraIncludes,
    excludes: cfg.excludes,
    skipScripts: cfg.skipScripts,
    skipFlows: cfg.skipFlows,
    skipApps: cfg.skipApps,
    skipFolders: cfg.skipFolders,
    skipVariables: cfg.skipVariables,
    skipResources: cfg.skipResources,
    skipResourceTypes: cfg.skipResourceTypes,
    skipSecrets: cfg.skipSecrets,
    includeSchedules: cfg.includeSchedules,
    includeTriggers: cfg.includeTriggers,
    includeUsers: cfg.includeUsers,
    includeGroups: cfg.includeGroups,
    includeSettings: cfg.includeSettings,
    includeKey: cfg.includeKey,
  } as SyncOptions;
}


// Keep backend values but preserve local-only fields that the backend doesn't track
export function mergeBackendSettingsWithLocalCodebases(backend: SyncOptions, local: SyncOptions): SyncOptions {
  return { 
    ...backend, 
    // Preserve local-only fields that backend doesn't track
    codebases: local.codebases || [],
    defaultTs: local.defaultTs, // TypeScript/Deno preference
    // Add any other local-only fields as needed
  };
}

