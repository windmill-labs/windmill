import { minimatch } from "minimatch";
import { getCurrentGitBranch, isGitRepository } from "../utils/git.ts";
import { isFileResource, isFilesetResource } from "../utils/utils.ts";
import { SyncOptions } from "./conf.ts";
import { TRIGGER_TYPES } from "../types.ts";

export interface SpecificItemsConfig {
  variables?: string[];
  resources?: string[];
  triggers?: string[];
  folders?: string[];
  settings?: boolean;
}

// Define all branch-specific file types (computed lazily)
function getBranchSpecificTypes() {
  return {
    variable: '.variable.yaml',
    resource: '.resource.yaml',
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
 * Extract the file type suffix from a path
 */
function getFileTypeSuffix(path: string): string | null {
  for (const [_, suffix] of Object.entries(getBranchSpecificTypes())) {
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
  const basicTypes = ['variable', 'resource'];
  const triggerTypes = TRIGGER_TYPES.map(t => `${t}_trigger`);
  return `((${basicTypes.join('|')})|(${triggerTypes.join('|')}))`;
}

/**
 * Get the specific items configuration for the current git branch
 * Merges commonSpecificItems with branch-specific specificItems
 */
export function getSpecificItemsForCurrentBranch(config: SyncOptions, branchOverride?: string): SpecificItemsConfig | undefined {
  if (!config.gitBranches) {
    return undefined;
  }

  // Use branch override if provided, otherwise detect from git
  let currentBranch: string | null = null;
  if (branchOverride) {
    currentBranch = branchOverride;
  } else if (isGitRepository()) {
    currentBranch = getCurrentGitBranch();
  }

  if (!currentBranch) {
    return undefined;
  }

  const commonItems = config.gitBranches.commonSpecificItems;
  const branchItems = config.gitBranches[currentBranch]?.specificItems;

  // If neither common nor branch-specific items exist, return undefined
  if (!commonItems && !branchItems) {
    return undefined;
  }

  // Merge common and branch-specific items
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
  if (commonItems?.folders) {
    merged.folders = [...commonItems.folders];
  }
  if (commonItems?.settings !== undefined) {
    merged.settings = commonItems.settings;
  }

  // Add branch-specific items (extending common items)
  if (branchItems?.variables) {
    merged.variables = [...(merged.variables || []), ...branchItems.variables];
  }
  if (branchItems?.resources) {
    merged.resources = [...(merged.resources || []), ...branchItems.resources];
  }
  if (branchItems?.triggers) {
    merged.triggers = [...(merged.triggers || []), ...branchItems.triggers];
  }
  if (branchItems?.folders) {
    merged.folders = [...(merged.folders || []), ...branchItems.folders];
  }
  // For settings (boolean), branch-specific overrides common
  if (branchItems?.settings !== undefined) {
    merged.settings = branchItems.settings;
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
 * Used to determine if branch-specific files should be used for this type.
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
 * Check if a file path should be treated as branch-specific
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
 * Convert a base path to a branch-specific path
 */
export function toBranchSpecificPath(basePath: string, branchName: string): string {
  // Sanitize branch name to be filesystem-safe
  const sanitizedBranchName = branchName.replace(/[\/\\:*?"<>|.]/g, '_');

  // Warn about potential collisions if sanitization occurred
  if (sanitizedBranchName !== branchName) {
    console.warn(`Warning: Branch name "${branchName}" contains filesystem-unsafe characters (/ \\ : * ? " < > | .) and was sanitized to "${sanitizedBranchName}". This may cause collisions with other similarly named branches.`);
  }

  // Check for folder meta file pattern: folder.meta.yaml -> folder.branchName.meta.yaml
  if (basePath.endsWith('/folder.meta.yaml')) {
    const pathWithoutMeta = basePath.substring(0, basePath.length - '/folder.meta.yaml'.length);
    return `${pathWithoutMeta}/folder.${sanitizedBranchName}.meta.yaml`;
  }

  // Check for settings.yaml: settings.yaml -> settings.branchName.yaml
  if (basePath === 'settings.yaml') {
    return `settings.${sanitizedBranchName}.yaml`;
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

  return `${pathWithoutExtension}.${sanitizedBranchName}${extension}`;
}

/**
 * Convert a branch-specific path back to a base path
 */
export function fromBranchSpecificPath(branchSpecificPath: string, branchName: string): string {
  // Sanitize branch name the same way as in toBranchSpecificPath
  const sanitizedBranchName = branchName.replace(/[\/\\:*?"<>|.]/g, '_');
  const escapedBranchName = sanitizedBranchName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

  // Check for folder meta file pattern: /folder.branchName.meta.yaml -> /folder.meta.yaml
  const folderPattern = new RegExp(`/folder\\.${escapedBranchName}\\.meta\\.yaml$`);
  if (folderPattern.test(branchSpecificPath)) {
    return branchSpecificPath.replace(folderPattern, '/folder.meta.yaml');
  }

  // Check for settings file pattern: settings.branchName.yaml -> settings.yaml
  const settingsPattern = new RegExp(`^settings\\.${escapedBranchName}\\.yaml$`);
  if (settingsPattern.test(branchSpecificPath)) {
    return 'settings.yaml';
  }

  // Check for resource file pattern
  const resourceFilePattern = new RegExp(`\\.${escapedBranchName}(\\.resource\\.file\\..+)$`);
  const resourceFileMatch = branchSpecificPath.match(resourceFilePattern);

  if (resourceFileMatch) {
    const extension = resourceFileMatch[1];
    const pathWithoutBranchAndExtension = branchSpecificPath.substring(
      0,
      branchSpecificPath.length - `.${sanitizedBranchName}${extension}`.length
    );
    return `${pathWithoutBranchAndExtension}${extension}`;
  }

  const yamlPattern = new RegExp(`\\.${escapedBranchName}(\\.${buildYamlTypePattern()}\\.yaml)$`);
  const yamlMatch = branchSpecificPath.match(yamlPattern);

  if (!yamlMatch) {
    return branchSpecificPath; // Return unchanged if not a branch-specific path
  }

  const extension = yamlMatch[1];
  const pathWithoutBranchAndExtension = branchSpecificPath.substring(
    0,
    branchSpecificPath.length - `.${sanitizedBranchName}${extension}`.length
  );

  return `${pathWithoutBranchAndExtension}${extension}`;
}

/**
 * Get the branch-specific path for the current branch if the item should be branch-specific
 */
export function getBranchSpecificPath(
  basePath: string,
  specificItems: SpecificItemsConfig | undefined,
  branchOverride?: string
): string | undefined {
  if (!specificItems) {
    return undefined;
  }

  // Use branch override if provided, otherwise detect from git
  let currentBranch: string | null = null;
  if (branchOverride) {
    currentBranch = branchOverride;
  } else if (isGitRepository()) {
    currentBranch = getCurrentGitBranch();
  }

  if (!currentBranch) {
    return undefined;
  }

  if (isSpecificItem(basePath, specificItems)) {
    return toBranchSpecificPath(basePath, currentBranch);
  }

  return undefined;
}

// Cache for compiled regex patterns to avoid recompilation
const branchPatternCache = new Map<string, RegExp>();

/**
 * Check if a path is a branch-specific file for the current branch
 */
export function isCurrentBranchFile(path: string, branchOverride?: string): boolean {
  // Use branch override if provided, otherwise detect from git
  let currentBranch: string | null = null;
  if (branchOverride) {
    currentBranch = branchOverride;
  } else if (isGitRepository()) {
    currentBranch = getCurrentGitBranch();
  }

  if (!currentBranch) {
    return false;
  }

  // Sanitize branch name to match what would be used in file naming
  const sanitizedBranchName = currentBranch.replace(/[\/\\:*?"<>|.]/g, '_');
  const escapedBranchName = sanitizedBranchName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

  // Use cached pattern or create and cache new one
  let pattern = branchPatternCache.get(currentBranch);
  if (!pattern) {
    pattern = new RegExp(
      `\\.${escapedBranchName}\\.${buildYamlTypePattern()}\\.yaml$|` +
      `\\.${escapedBranchName}\\.resource\\.file\\..+$|` +
      `/folder\\.${escapedBranchName}\\.meta\\.yaml$|` +
      `^settings\\.${escapedBranchName}\\.yaml$`
    );
    branchPatternCache.set(currentBranch, pattern);
  }

  return pattern.test(path);
}

/**
 * Check if a path is a branch-specific file for ANY branch (not necessarily current)
 * Used to identify and skip files from other branches during sync operations
 */
export function isBranchSpecificFile(path: string): boolean {
  const yamlTypePattern = buildYamlTypePattern();
  return new RegExp(
    `\\.[^.]+\\.${yamlTypePattern}\\.yaml$|` +
    `\\.[^.]+\\.resource\\.file\\..+$|` +
    `/folder\\.[^.]+\\.meta\\.yaml$|` +
    `^settings\\.[^.]+\\.yaml$`
  ).test(path);
}
