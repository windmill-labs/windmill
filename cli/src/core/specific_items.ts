import { minimatch } from "../../deps.ts";
import { getCurrentGitBranch, isGitRepository } from "../utils/git.ts";
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

  return false;
}

/**
 * Convert a base path to a branch-specific path
 */
export function toBranchSpecificPath(basePath: string, branchName: string): string {
  // Extract the extension (e.g., ".variable.yaml" or ".resource.yaml")
  const extensionMatch = basePath.match(/(\.(variable|resource)\.yaml)$/);
  if (!extensionMatch) {
    return basePath; // Return unchanged if no recognized extension
  }

  const extension = extensionMatch[1];
  const pathWithoutExtension = basePath.substring(0, basePath.length - extension.length);

  // Sanitize branch name to be filesystem-safe
  const sanitizedBranchName = branchName.replace(/[\/\\:*?"<>|]/g, '_');

  // Warn about potential collisions if sanitization occurred
  if (sanitizedBranchName !== branchName) {
    console.warn(`Warning: Branch name "${branchName}" contains special characters and was sanitized to "${sanitizedBranchName}". This may cause collisions with other similarly named branches.`);
  }

  return `${pathWithoutExtension}.${sanitizedBranchName}${extension}`;
}

/**
 * Convert a branch-specific path back to a base path
 */
export function fromBranchSpecificPath(branchSpecificPath: string, branchName: string): string {
  // Sanitize branch name the same way as in toBranchSpecificPath
  const sanitizedBranchName = branchName.replace(/[\/\\:*?"<>|]/g, '_');

  // Pattern: path.sanitizedBranchName.extension
  const escapedBranchName = sanitizedBranchName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
  const pattern = new RegExp(`\\.${escapedBranchName}(\\.(variable|resource)\\.yaml)$`);
  const match = branchSpecificPath.match(pattern);

  if (!match) {
    return branchSpecificPath; // Return unchanged if not a branch-specific path
  }

  const extension = match[1];
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

  // Use cached pattern or create and cache new one
  let pattern = branchPatternCache.get(currentBranch);
  if (!pattern) {
    pattern = new RegExp(`\\.${currentBranch}\\.(variable|resource)\\.yaml$`);
    branchPatternCache.set(currentBranch, pattern);
  }

  return pattern.test(path);
}

/**
 * Check if a path is a branch-specific file for ANY branch (not necessarily current)
 * Used to identify and skip files from other branches during sync operations
 */
export function isBranchSpecificFile(path: string): boolean {
  // Pattern: *.branchName.variable.yaml or *.branchName.resource.yaml
  return /\.[^.]+\.(variable|resource)\.yaml$/.test(path);
}
