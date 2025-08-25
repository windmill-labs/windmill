import { minimatch } from "../../deps.ts";
import { getCurrentGitBranch, isGitRepository } from "../utils/git.ts";
import { isFileResource } from "../utils/utils.ts";
import { SyncOptions } from "./conf.ts";

export interface SpecificItemsConfig {
  variables?: string[];
  resources?: string[];
}

/**
 * Get the specific items configuration for the current git branch
 * Merges commonSpecificItems with branch-specific specificItems
 */
export function getSpecificItemsForCurrentBranch(config: SyncOptions): SpecificItemsConfig | undefined {
  if (!isGitRepository() || !config.gitBranches) {
    return undefined;
  }

  const currentBranch = getCurrentGitBranch();
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

  // Add branch-specific items (extending common items)
  if (branchItems?.variables) {
    merged.variables = [...(merged.variables || []), ...branchItems.variables];
  }
  if (branchItems?.resources) {
    merged.resources = [...(merged.resources || []), ...branchItems.resources];
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

  // Check for resource files using the standard detection function
  if (isFileResource(path)) {
    // Extract the base path without the file extension to match against patterns  
    const basePathMatch = path.match(/^(.+?)\.resource\.file\./);
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
  // Check for resource file pattern (e.g., .resource.file.ini)
  const resourceFileMatch = basePath.match(/^(.+?)(\.resource\.file\..+)$/);

  let extension: string;
  let pathWithoutExtension: string;

  if (resourceFileMatch) {
    // Handle resource files
    extension = resourceFileMatch[2];
    pathWithoutExtension = resourceFileMatch[1];
  } else {
    // Extract the extension (e.g., ".variable.yaml" or ".resource.yaml")
    const extensionMatch = basePath.match(/(\.(variable|resource)\.yaml)$/);
    if (!extensionMatch) {
      return basePath; // Return unchanged if no recognized extension
    }
    extension = extensionMatch[1];
    pathWithoutExtension = basePath.substring(0, basePath.length - extension.length);
  }

  // Sanitize branch name to be filesystem-safe
  const sanitizedBranchName = branchName.replace(/[\/\\:*?"<>|.]/g, '_');

  // Warn about potential collisions if sanitization occurred
  if (sanitizedBranchName !== branchName) {
    console.warn(`Warning: Branch name "${branchName}" contains filesystem-unsafe characters (/ \\ : * ? " < > | .) and was sanitized to "${sanitizedBranchName}". This may cause collisions with other similarly named branches.`);
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

  // Check for resource file pattern first
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

  // Pattern for regular yaml files: path.sanitizedBranchName.extension
  const yamlPattern = new RegExp(`\\.${escapedBranchName}(\\.(variable|resource)\\.yaml)$`);
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
  specificItems: SpecificItemsConfig | undefined
): string | undefined {
  if (!isGitRepository() || !specificItems) {
    return undefined;
  }

  const currentBranch = getCurrentGitBranch();
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
export function isCurrentBranchFile(path: string): boolean {
  if (!isGitRepository()) {
    return false;
  }

  const currentBranch = getCurrentGitBranch();
  if (!currentBranch) {
    return false;
  }

  // Sanitize branch name to match what would be used in file naming
  const sanitizedBranchName = currentBranch.replace(/[\/\\:*?"<>|.]/g, '_');
  const escapedBranchName = sanitizedBranchName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

  // Use cached pattern or create and cache new one
  let pattern = branchPatternCache.get(currentBranch);
  if (!pattern) {
    // Match branch-specific files: *.{branch}.(variable|resource).yaml OR *.{branch}.resource.file.{ext}
    // Examples: file.dev.variable.yaml, config.main.resource.yaml, cert.dev2.resource.file.pem
    pattern = new RegExp(`\\.${escapedBranchName}\\.(variable|resource)\\.yaml$|\\.${escapedBranchName}\\.resource\\.file\\..+$`);
    branchPatternCache.set(currentBranch, pattern);
  }

  return pattern.test(path);
}

/**
 * Check if a path is a branch-specific file for ANY branch (not necessarily current)
 * Used to identify and skip files from other branches during sync operations
 */
export function isBranchSpecificFile(path: string): boolean {
  // Match any branch-specific file pattern using generic branch name matching
  // Pattern matches: *.{anyBranch}.(variable|resource).yaml OR *.{anyBranch}.resource.file.{ext}
  // [^.]+ captures any branch name (non-dot characters), examples: dev, main, feature_branch
  return /\.[^.]+\.(variable|resource)\.yaml$|\.[^.]+\.resource\.file\..+$/.test(path);
}
