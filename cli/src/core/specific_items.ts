import { minimatch } from "minimatch";
import { getCurrentGitBranch, isGitRepository } from "../utils/git.ts";
import { isFileResource, isFilesetResource } from "../utils/utils.ts";
import {
  SyncOptions,
  findWorkspaceByGitBranch,
  WorkspaceEntryConfig,
} from "./conf.ts";
import { TRIGGER_TYPES } from "../types.ts";

export interface SpecificItemsConfig {
  variables?: string[];
  resources?: string[];
  triggers?: string[];
  schedules?: string[];
  folders?: string[];
  settings?: boolean;
}

// Define all workspace-specific file types (computed lazily)
function getWorkspaceSpecificTypes() {
  return {
    variable: '.variable.yaml',
    resource: '.resource.yaml',
    schedule: '.schedule.yaml',
    // Generate trigger patterns from the list
    ...Object.fromEntries(
      TRIGGER_TYPES.map(t => [`${t}_trigger`, `.${t}_trigger.yaml`])
    )
  } as const;
}

/**
 * Check if a path ends with any trigger type
 */
function isTriggerFile(path: string): boolean {
  return TRIGGER_TYPES.some(type => path.endsWith(`.${type}_trigger.yaml`));
}

/**
 * Check if a path is a schedule file
 */
function isScheduleFile(path: string): boolean {
  return path.endsWith('.schedule.yaml');
}

/**
 * Extract the file type suffix from a path
 */
function getFileTypeSuffix(path: string): string | null {
  for (const [_, suffix] of Object.entries(getWorkspaceSpecificTypes())) {
    if (path.endsWith(suffix)) {
      return suffix;
    }
  }

  const resourceFileMatch = path.match(/(\\.resource\\.file\\..+)$/);
  if (resourceFileMatch) {
    return resourceFileMatch[1];
  }

  return null;
}

/**
 * Build regex pattern for all supported yaml file types
 */
function buildYamlTypePattern(): string {
  const basicTypes = ['variable', 'resource', 'schedule'];
  const triggerTypes = TRIGGER_TYPES.map(t => `${t}_trigger`);
  return `((${basicTypes.join('|')})|(${triggerTypes.join('|')}))`;
}

/**
 * Get the specific items configuration for the current workspace.
 * workspaceNameOverride selects by workspace name (O(1)).
 * When not provided, auto-detects from the current git branch.
 * Merges commonSpecificItems with workspace-specific specificItems.
 */
export function getSpecificItemsForCurrentBranch(config: SyncOptions, workspaceNameOverride?: string): SpecificItemsConfig | undefined {
  if (!config.workspaces) {
    return undefined;
  }

  let wsEntry: WorkspaceEntryConfig | undefined;

  if (workspaceNameOverride) {
    wsEntry = config.workspaces[workspaceNameOverride] as WorkspaceEntryConfig | undefined;
  } else if (isGitRepository()) {
    const currentWorkspace = getCurrentGitBranch();
    if (currentWorkspace) {
      const match = findWorkspaceByGitBranch(config.workspaces, currentWorkspace);
      if (match) {
        wsEntry = match[1];
      }
    }
  }

  const commonItems = config.workspaces.commonSpecificItems;
  const wsItems = wsEntry?.specificItems;

  // If neither common nor workspace-specific items exist, return undefined
  if (!commonItems && !wsItems) {
    return undefined;
  }

  // Merge common and workspace-specific items
  const merged: SpecificItemsConfig = {};

  // Add common items
  if (commonItems?.variables) {
    merged.variables = [...commonItems.variables];
  }
  if (commonItems?.resources) {
    merged.resources = [...commonItems.resources];
  }
  if (commonItems?.triggers) {
    merged.triggers = [...commonItems.triggers];
  }
  if (commonItems?.schedules) {
    merged.schedules = [...commonItems.schedules];
  }
  if (commonItems?.folders) {
    merged.folders = [...commonItems.folders];
  }
  if (commonItems?.settings !== undefined) {
    merged.settings = commonItems.settings;
  }

  // Add workspace-specific items (extending common items)
  if (wsItems?.variables) {
    merged.variables = [...(merged.variables || []), ...wsItems.variables];
  }
  if (wsItems?.resources) {
    merged.resources = [...(merged.resources || []), ...wsItems.resources];
  }
  if (wsItems?.triggers) {
    merged.triggers = [...(merged.triggers || []), ...wsItems.triggers];
  }
  if (wsItems?.schedules) {
    merged.schedules = [...(merged.schedules || []), ...wsItems.schedules];
  }
  if (wsItems?.folders) {
    merged.folders = [...(merged.folders || []), ...wsItems.folders];
  }
  // For settings (boolean), workspace-specific overrides common
  if (wsItems?.settings !== undefined) {
    merged.settings = wsItems.settings;
  }

  return merged;
}

/**
 * Check if a path matches any of the patterns in the given list
 */
function matchesPatterns(path: string, patterns: string[]): boolean {
  return patterns.some(pattern => minimatch(path, pattern));
}

/**
 * Check if the item type for a given path is configured in specificItems.
 * This checks if the TYPE is configured, not whether it matches the pattern.
 * Used to determine if workspace-specific files should be used for this type.
 */
export function isItemTypeConfigured(path: string, specificItems: SpecificItemsConfig | undefined): boolean {
  if (!specificItems) {
    return false;
  }

  if (path.endsWith('.variable.yaml')) {
    return specificItems.variables !== undefined;
  }

  if (path.endsWith('.resource.yaml')) {
    return specificItems.resources !== undefined;
  }

  if (isTriggerFile(path)) {
    return specificItems.triggers !== undefined;
  }

  if (isScheduleFile(path)) {
    return specificItems.schedules !== undefined;
  }

  if (path.endsWith('/folder.meta.yaml')) {
    return specificItems.folders !== undefined;
  }

  if (path === 'settings.yaml') {
    return specificItems.settings !== undefined;
  }

  if (isFileResource(path) || isFilesetResource(path)) {
    return specificItems.resources !== undefined;
  }

  return false;
}

/**
 * Check if a file path should be treated as workspace-specific
 */
export function isSpecificItem(path: string, specificItems: SpecificItemsConfig | undefined): boolean {
  if (!specificItems) {
    return false;
  }

  // Determine the item type from the file path
  if (path.endsWith('.variable.yaml')) {
    return specificItems.variables ? matchesPatterns(path, specificItems.variables) : false;
  }

  if (path.endsWith('.resource.yaml')) {
    return specificItems.resources ? matchesPatterns(path, specificItems.resources) : false;
  }

  // Check for any trigger type
  if (isTriggerFile(path)) {
    return specificItems.triggers ? matchesPatterns(path, specificItems.triggers) : false;
  }

  // Check for schedule files
  if (isScheduleFile(path)) {
    return specificItems.schedules ? matchesPatterns(path, specificItems.schedules) : false;
  }

  // Check for folder meta files
  if (path.endsWith('/folder.meta.yaml')) {
    if (specificItems.folders) {
      // Match against the folder path (without /folder.meta.yaml)
      const folderPath = path.slice(0, -'/folder.meta.yaml'.length);
      return matchesPatterns(folderPath, specificItems.folders);
    }
    return false;
  }

  // Check for settings.yaml (root-level file)
  if (path === 'settings.yaml') {
    return specificItems.settings === true;
  }

  // Check for resource files using the standard detection function
  if (isFileResource(path)) {
    // Extract the base path without the file extension to match against patterns
    const basePathMatch = path.match(/^(.+?)\.resource\.file\./);
    if (basePathMatch && specificItems.resources) {
      const basePath = basePathMatch[1] + '.resource.yaml';
      return matchesPatterns(basePath, specificItems.resources);
    }
  }

  if (isFilesetResource(path)) {
    const basePathMatch = path.match(/^(.+?)\.fileset[/\\]/);
    if (basePathMatch && specificItems.resources) {
      const basePath = basePathMatch[1] + '.resource.yaml';
      return matchesPatterns(basePath, specificItems.resources);
    }
  }

  return false;
}

/**
 * Convert a base path to a workspace-specific path
 */
export function toWorkspaceSpecificPath(basePath: string, workspaceName: string): string {
  // Sanitize branch name to be filesystem-safe
  const sanitizedName = workspaceName.replace(/[\/\\:*?"<>|.]/g, '_');

  // Warn about potential collisions if sanitization occurred
  if (sanitizedName !== workspaceName) {
    console.warn(`Warning: Workspace name "${workspaceName}" contains filesystem-unsafe characters (/ \\ : * ? " < > | .) and was sanitized to "${sanitizedName}". This may cause collisions with other similarly named branches.`);
  }

  // Check for folder meta file pattern: folder.meta.yaml -> folder.workspaceName.meta.yaml
  if (basePath.endsWith('/folder.meta.yaml')) {
    const pathWithoutMeta = basePath.substring(0, basePath.length - '/folder.meta.yaml'.length);
    return `${pathWithoutMeta}/folder.${sanitizedName}.meta.yaml`;
  }

  // Check for settings.yaml: settings.yaml -> settings.workspaceName.yaml
  if (basePath === 'settings.yaml') {
    return `settings.${sanitizedName}.yaml`;
  }

  // Check for resource file pattern (e.g., .resource.file.ini)
  const resourceFileMatch = basePath.match(/^(.+?)(\.resource\.file\..+)$/);

  let extension: string;
  let pathWithoutExtension: string;

  if (resourceFileMatch) {
    // Handle resource files
    extension = resourceFileMatch[2];
    pathWithoutExtension = resourceFileMatch[1];
  } else {
    const suffix = getFileTypeSuffix(basePath);
    if (!suffix) {
      return basePath;
    }
    extension = suffix;
    pathWithoutExtension = basePath.substring(0, basePath.length - extension.length);
  }

  return `${pathWithoutExtension}.${sanitizedName}${extension}`;
}

/**
 * Convert a workspace-specific path back to a base path
 */
export function fromWorkspaceSpecificPath(workspaceSpecificPath: string, workspaceName: string): string {
  // Sanitize branch name the same way as in toWorkspaceSpecificPath
  const sanitizedName = workspaceName.replace(/[\/\\:*?"<>|.]/g, '_');
  const escapedName = sanitizedName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

  // Check for folder meta file pattern: /folder.workspaceName.meta.yaml -> /folder.meta.yaml
  const folderPattern = new RegExp(`/folder\\.${escapedName}\\.meta\\.yaml$`);
  if (folderPattern.test(workspaceSpecificPath)) {
    return workspaceSpecificPath.replace(folderPattern, '/folder.meta.yaml');
  }

  // Check for settings file pattern: settings.workspaceName.yaml -> settings.yaml
  const settingsPattern = new RegExp(`^settings\\.${escapedName}\\.yaml$`);
  if (settingsPattern.test(workspaceSpecificPath)) {
    return 'settings.yaml';
  }

  // Check for resource file pattern
  const resourceFilePattern = new RegExp(`\\.${escapedName}(\\.resource\\.file\\..+)$`);
  const resourceFileMatch = workspaceSpecificPath.match(resourceFilePattern);

  if (resourceFileMatch) {
    const extension = resourceFileMatch[1];
    const pathWithoutBranchAndExtension = workspaceSpecificPath.substring(
      0,
      workspaceSpecificPath.length - `.${sanitizedName}${extension}`.length
    );
    return `${pathWithoutBranchAndExtension}${extension}`;
  }

  const yamlPattern = new RegExp(`\\.${escapedName}(\\.${buildYamlTypePattern()}\\.yaml)$`);
  const yamlMatch = workspaceSpecificPath.match(yamlPattern);

  if (!yamlMatch) {
    return workspaceSpecificPath; // Return unchanged if not a workspace-specific path
  }

  const extension = yamlMatch[1];
  const pathWithoutBranchAndExtension = workspaceSpecificPath.substring(
    0,
    workspaceSpecificPath.length - `.${sanitizedName}${extension}`.length
  );

  return `${pathWithoutBranchAndExtension}${extension}`;
}

/**
 * Get the workspace-specific path if the item should be workspace-specific.
 * workspaceNameOverride is the workspace name used as file suffix.
 * Falls back to current git branch when no override (backward compat: old key = branch name).
 */
export function getWorkspaceSpecificPath(
  basePath: string,
  specificItems: SpecificItemsConfig | undefined,
  workspaceNameOverride?: string
): string | undefined {
  if (!specificItems) {
    return undefined;
  }

  let currentWorkspace: string | null = null;
  if (workspaceNameOverride) {
    currentWorkspace = workspaceNameOverride;
  } else if (isGitRepository()) {
    currentWorkspace = getCurrentGitBranch();
  }

  if (!currentWorkspace) {
    return undefined;
  }

  if (isSpecificItem(basePath, specificItems)) {
    return toWorkspaceSpecificPath(basePath, currentWorkspace);
  }

  return undefined;
}

// Cache for compiled regex patterns to avoid recompilation
const workspacePatternCache = new Map<string, RegExp>();

/**
 * Check if a path is a workspace-specific file for the current branch.
 * workspaceNameOverride is the effective git branch name (for file naming on disk).
 */
export function isCurrentWorkspaceFile(path: string, workspaceNameOverride?: string): boolean {
  let currentWorkspace: string | null = null;
  if (workspaceNameOverride) {
    currentWorkspace = workspaceNameOverride;
  } else if (isGitRepository()) {
    currentWorkspace = getCurrentGitBranch();
  }

  if (!currentWorkspace) {
    return false;
  }

  // Sanitize branch name to match what would be used in file naming
  const sanitizedName = currentWorkspace.replace(/[\/\\:*?"<>|.]/g, '_');
  const escapedName = sanitizedName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

  // Use cached pattern or create and cache new one
  let pattern = workspacePatternCache.get(currentWorkspace);
  if (!pattern) {
    pattern = new RegExp(
      `\\.${escapedName}\\.${buildYamlTypePattern()}\\.yaml$|` +
      `\\.${escapedName}\\.resource\\.file\\..+$|` +
      `/folder\\.${escapedName}\\.meta\\.yaml$|` +
      `^settings\\.${escapedName}\\.yaml$`
    );
    workspacePatternCache.set(currentWorkspace, pattern);
  }

  return pattern.test(path);
}

/**
 * Check if a path is a workspace-specific file for ANY branch (not necessarily current)
 * Used to identify and skip files from other branches during sync operations
 */
export function isWorkspaceSpecificFile(path: string): boolean {
  const yamlTypePattern = buildYamlTypePattern();
  return new RegExp(
    `\\.[^.]+\\.${yamlTypePattern}\\.yaml$|` +
    `\\.[^.]+\\.resource\\.file\\..+$|` +
    `/folder\\.[^.]+\\.meta\\.yaml$|` +
    `^settings\\.[^.]+\\.yaml$`
  ).test(path);
}
