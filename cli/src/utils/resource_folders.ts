/**
 * Utility module for handling resource folder naming conventions.
 *
 * This module centralizes the logic for detecting and manipulating paths
 * that contain resource folders (.flow, .app, .raw_app).
 *
 * The folder suffixes can be configured to use either dot-prefixed names
 * (.flow, .app, .raw_app) or dunder-prefixed names (__flow, __app, __raw_app).
 */

import * as log from "@std/log";
import { SEPARATOR as SEP } from "@std/path";
import { yamlParseFile } from "./yaml.ts";
import * as fs from "node:fs";
import * as path from "node:path";
import process from "node:process";

// Resource types that use folder-based storage
export type FolderResourceType = "flow" | "app" | "raw_app";

// Configuration for folder suffixes - can be switched between dot and dunder prefixes
// The default uses dot-prefixed names (.flow, .app, .raw_app)
const DOTTED_SUFFIXES = {
  flow: ".flow",
  app: ".app",
  raw_app: ".raw_app",
} as const;

// Alternative dunder-prefixed names (__flow, __app, __raw_app)
const NON_DOTTED_SUFFIXES = {
  flow: "__flow",
  app: "__app",
  raw_app: "__raw_app",
} as const;

export type FolderSuffixes =
  | typeof DOTTED_SUFFIXES
  | typeof NON_DOTTED_SUFFIXES;

// Global state for nonDottedPaths configuration
let _nonDottedPaths = false;
let _nonDottedPathsLogged = false;

/**
 * Set whether to use non-dotted paths (__flow, __app, __raw_app)
 * instead of dotted paths (.flow, .app, .raw_app).
 * This should be called once at startup based on wmill.yaml configuration.
 */
export function setNonDottedPaths(value: boolean): void {
  if (value && !_nonDottedPathsLogged) {
    log.info("Using non-dotted paths (__flow, __app, __raw_app)");
    _nonDottedPathsLogged = true;
  }
  _nonDottedPaths = value;
}

/**
 * Get the current nonDottedPaths setting.
 */
export function getNonDottedPaths(): boolean {
  return _nonDottedPaths;
}

/**
 * Search for wmill.yaml by traversing upward from the current directory
 * and initialize the nonDottedPaths setting.
 * Unlike findWmillYaml() in conf.ts, this does not stop at the git root -
 * it continues searching until the filesystem root.
 * This is needed for commands like `app dev` and `app new` which may run
 * from inside folders that are deeply nested within a larger git repository.
 */
export async function loadNonDottedPathsSetting(): Promise<void> {
  let currentDir = process.cwd();

  while (true) {
    const wmillYamlPath = path.join(currentDir, "wmill.yaml");

    if (fs.existsSync(wmillYamlPath)) {
      try {
        const config = (await yamlParseFile(wmillYamlPath)) as {
          nonDottedPaths?: boolean;
        };
        setNonDottedPaths(config?.nonDottedPaths ?? false);
        log.debug(
          `Found wmill.yaml at ${wmillYamlPath}, nonDottedPaths=${
            config?.nonDottedPaths ?? false
          }`
        );
      } catch (e) {
        log.debug(`Failed to parse wmill.yaml at ${wmillYamlPath}: ${e}`);
      }
      return;
    }

    // Check if we've reached the filesystem root
    const parentDir = path.dirname(currentDir);
    if (parentDir === currentDir) {
      // Reached filesystem root without finding wmill.yaml
      log.debug("No wmill.yaml found, using default dotted paths");
      return;
    }

    currentDir = parentDir;
  }
}

/**
 * Get the folder suffixes based on the global configuration.
 */
export function getFolderSuffixes(): FolderSuffixes {
  return _nonDottedPaths ? NON_DOTTED_SUFFIXES : DOTTED_SUFFIXES;
}

// Metadata file names inside each folder type
const METADATA_FILES = {
  flow: { yaml: "flow.yaml", json: "flow.json" },
  app: { yaml: "app.yaml", json: "app.json" },
  raw_app: { yaml: "raw_app.yaml", json: "raw_app.json" },
} as const;

/**
 * Get the folder suffix for a resource type (e.g., ".flow", ".app", ".raw_app" or "__flow", "__app", "__raw_app")
 */
export function getFolderSuffix(type: FolderResourceType): string {
  return getFolderSuffixes()[type];
}

/**
 * Get the folder suffix with path separator (e.g., ".flow/", ".app/", ".raw_app/")
 */
export function getFolderSuffixWithSep(type: FolderResourceType): string {
  return getFolderSuffixes()[type] + SEP;
}

/**
 * Get the metadata file name for a resource type
 */
export function getMetadataFileName(
  type: FolderResourceType,
  format: "yaml" | "json"
): string {
  return METADATA_FILES[type][format];
}

/**
 * Get the full metadata file path suffix (e.g., ".flow/flow.yaml" or "__flow/flow.yaml")
 */
export function getMetadataPathSuffix(
  type: FolderResourceType,
  format: "yaml" | "json"
): string {
  return getFolderSuffixes()[type] + "/" + METADATA_FILES[type][format];
}

// ============================================================================
// Path Detection Functions
// ============================================================================

/** Normalize path separators to forward slash for cross-platform matching */
function normalizeSep(p: string): string {
  return p.replaceAll("\\", "/");
}

/**
 * Check if a path is inside a flow folder
 */
export function isFlowPath(p: string): boolean {
  return normalizeSep(p).includes(getFolderSuffixes().flow + "/");
}

/**
 * Check if a path is inside an app folder
 */
export function isAppPath(p: string): boolean {
  return normalizeSep(p).includes(getFolderSuffixes().app + "/");
}

/**
 * Check if a path is inside a raw_app folder
 */
export function isRawAppPath(p: string): boolean {
  return normalizeSep(p).includes(getFolderSuffixes().raw_app + "/");
}

/**
 * Check if a path is inside any folder-based resource (flow, app, or raw_app)
 */
export function isFolderResourcePath(p: string): boolean {
  return isFlowPath(p) || isAppPath(p) || isRawAppPath(p);
}

/**
 * Detect the resource type from a path, if any
 */
export function detectFolderResourceType(p: string): FolderResourceType | null {
  if (isFlowPath(p)) return "flow";
  if (isAppPath(p)) return "app";
  if (isRawAppPath(p)) return "raw_app";
  return null;
}

/**
 * Check if a path is inside a raw app backend folder.
 * Matches patterns like: .../myApp.raw_app/backend/... or .../myApp__raw_app/backend/...
 */
export function isRawAppBackendPath(filePath: string): boolean {
  const suffixes = getFolderSuffixes();
  // Normalize path separators for consistent matching
  const normalizedPath = filePath.replaceAll(SEP, "/");
  // Check if path contains pattern: *.[suffix]/backend/
  const escapedSuffix = suffixes.raw_app.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const pattern = new RegExp(`${escapedSuffix}/backend/`);
  return pattern.test(normalizedPath);
}

/**
 * Check if a path is inside a normal app folder (inline script).
 * Matches patterns like: .../myApp.app/... or .../myApp__app/...
 * This is used to detect inline scripts that belong to normal apps.
 */
export function isAppInlineScriptPath(filePath: string): boolean {
  const suffixes = getFolderSuffixes();
  // Normalize path separators for consistent matching
  const normalizedPath = filePath.replaceAll(SEP, "/");
  // Check if path contains pattern: *.[suffix]/
  const escapedSuffix = suffixes.app.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const pattern = new RegExp(`${escapedSuffix}/`);
  return pattern.test(normalizedPath);
}

/**
 * Check if a path is inside a flow folder (inline script).
 * Matches patterns like: .../myFlow.flow/... or .../myFlow__flow/...
 * This is used to detect inline scripts that belong to flows.
 */
export function isFlowInlineScriptPath(filePath: string): boolean {
  const suffixes = getFolderSuffixes();
  // Normalize path separators for consistent matching
  const normalizedPath = filePath.replaceAll(SEP, "/");
  // Check if path contains pattern: *.[suffix]/
  const escapedSuffix = suffixes.flow.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const pattern = new RegExp(`${escapedSuffix}/`);
  return pattern.test(normalizedPath);
}

// ============================================================================
// Path Manipulation Functions
// ============================================================================

/**
 * Extract the resource name from a path (the part before the folder suffix)
 * e.g., "f/my_flow.flow/flow.yaml" -> "f/my_flow"
 */
export function extractResourceName(
  p: string,
  type: FolderResourceType
): string | null {
  const normalized = normalizeSep(p);
  const suffix = getFolderSuffixes()[type] + "/";
  const index = normalized.indexOf(suffix);
  if (index === -1) return null;
  return normalized.substring(0, index);
}

/**
 * Extract the folder path (resource name + folder suffix)
 * e.g., "f/my_flow.flow/flow.yaml" -> "f/my_flow.flow/"
 */
export function extractFolderPath(
  p: string,
  type: FolderResourceType
): string | null {
  const normalized = normalizeSep(p);
  const suffix = getFolderSuffixes()[type] + "/";
  const index = normalized.indexOf(suffix);
  if (index === -1) return null;
  return normalized.substring(0, index) + suffix;
}

/**
 * Build a folder path from a resource name
 * e.g., ("f/my_flow", "flow") -> "f/my_flow.flow"
 */
export function buildFolderPath(
  resourceName: string,
  type: FolderResourceType
): string {
  return resourceName + getFolderSuffixes()[type];
}

/**
 * Build a metadata file path from a resource name
 * e.g., ("f/my_flow", "flow", "yaml") -> "f/my_flow.flow/flow.yaml"
 */
export function buildMetadataPath(
  resourceName: string,
  type: FolderResourceType,
  format: "yaml" | "json"
): string {
  return (
    resourceName +
    getFolderSuffixes()[type] +
    "/" +
    METADATA_FILES[type][format]
  );
}

// ============================================================================
// Folder Validation Functions
// ============================================================================

/**
 * Check if a directory name ends with a specific resource folder suffix
 * e.g., "my_app.raw_app" ends with ".raw_app" or "my_app__raw_app" ends with "__raw_app"
 */
export function hasFolderSuffix(
  dirName: string,
  type: FolderResourceType
): boolean {
  return dirName.endsWith(getFolderSuffixes()[type]);
}

/**
 * Validate that a directory name has the expected folder suffix
 * Returns an error message if invalid, null if valid
 */
export function validateFolderName(
  dirName: string,
  type: FolderResourceType
): string | null {
  const suffixes = getFolderSuffixes();
  if (!hasFolderSuffix(dirName, type)) {
    return `'${dirName}' does not end with '${suffixes[type]}'`;
  }
  return null;
}

/**
 * Extract the resource name from a folder name by removing the suffix
 * e.g., "my_app.raw_app" -> "my_app" or "my_app__raw_app" -> "my_app"
 */
export function extractNameFromFolder(
  folderName: string,
  type: FolderResourceType
): string {
  const suffix = getFolderSuffixes()[type];
  if (folderName.endsWith(suffix)) {
    return folderName.substring(0, folderName.length - suffix.length);
  }
  return folderName;
}

// ============================================================================
// Metadata File Detection Functions
// ============================================================================

/**
 * Check if a path ends with a flow metadata file suffix.
 * Detects BOTH API format (always dotted: .flow.json) and local format (user-configured).
 * This is necessary because the API always returns dotted format, but local files
 * may use non-dotted format if nonDottedPaths is configured.
 */
export function isFlowMetadataFile(p: string): boolean {
  // Always check API format (dotted)
  if (
    p.endsWith(DOTTED_SUFFIXES.flow + ".json") ||
    p.endsWith(DOTTED_SUFFIXES.flow + ".yaml")
  ) {
    return true;
  }
  // Also check non-dotted format for local files
  if (_nonDottedPaths) {
    return (
      p.endsWith(NON_DOTTED_SUFFIXES.flow + ".json") ||
      p.endsWith(NON_DOTTED_SUFFIXES.flow + ".yaml")
    );
  }
  return false;
}

/**
 * Check if a path ends with an app metadata file suffix.
 * Detects BOTH API format (always dotted: .app.json) and local format (user-configured).
 */
export function isAppMetadataFile(p: string): boolean {
  // Always check API format (dotted)
  if (
    p.endsWith(DOTTED_SUFFIXES.app + ".json") ||
    p.endsWith(DOTTED_SUFFIXES.app + ".yaml")
  ) {
    return true;
  }
  // Also check non-dotted format for local files
  if (_nonDottedPaths) {
    return (
      p.endsWith(NON_DOTTED_SUFFIXES.app + ".json") ||
      p.endsWith(NON_DOTTED_SUFFIXES.app + ".yaml")
    );
  }
  return false;
}

/**
 * Check if a path ends with a raw_app metadata file suffix.
 * Detects BOTH API format (always dotted: .raw_app.json) and local format (user-configured).
 */
export function isRawAppMetadataFile(p: string): boolean {
  // Always check API format (dotted)
  if (
    p.endsWith(DOTTED_SUFFIXES.raw_app + ".json") ||
    p.endsWith(DOTTED_SUFFIXES.raw_app + ".yaml")
  ) {
    return true;
  }
  // Also check non-dotted format for local files
  if (_nonDottedPaths) {
    return (
      p.endsWith(NON_DOTTED_SUFFIXES.raw_app + ".json") ||
      p.endsWith(NON_DOTTED_SUFFIXES.raw_app + ".yaml")
    );
  }
  return false;
}

/**
 * Check if a path ends with a specific raw_app metadata file
 * (inside the folder, e.g., ".raw_app/raw_app.yaml" or "__raw_app/raw_app.yaml")
 */
export function isRawAppFolderMetadataFile(p: string): boolean {
  return (
    p.endsWith(getMetadataPathSuffix("raw_app", "yaml")) ||
    p.endsWith(getMetadataPathSuffix("raw_app", "json"))
  );
}

// ============================================================================
// Sync-related Path Functions
// ============================================================================

/**
 * Get the path suffix to remove when converting local path to API path
 * for delete operations
 */
export function getDeleteSuffix(
  type: FolderResourceType,
  format: "yaml" | "json"
): string {
  return getFolderSuffixes()[type] + "/" + METADATA_FILES[type][format];
}

/**
 * Transform a JSON path from API format to local directory path for sync.
 * The API always returns dotted format (.flow.json, .app.json, .raw_app.json).
 * This function transforms to the user's configured format (dotted or non-dotted).
 * e.g., with nonDottedPaths=true: "f/my_flow.flow.json" -> "f/my_flow__flow"
 * e.g., with nonDottedPaths=false: "f/my_flow.flow.json" -> "f/my_flow.flow"
 */
export function transformJsonPathToDir(
  p: string,
  type: FolderResourceType
): string {
  // API always returns dotted format
  const apiSuffix = DOTTED_SUFFIXES[type] + ".json";
  if (p.endsWith(apiSuffix)) {
    // Remove the API suffix and add user's configured suffix
    const basePath = p.substring(0, p.length - apiSuffix.length);
    return basePath + getFolderSuffixes()[type];
  }
  // Also handle the case where path already has user's configured format
  const userSuffix = getFolderSuffixes()[type] + ".json";
  if (p.endsWith(userSuffix)) {
    return p.substring(0, p.length - 5); // Remove ".json"
  }
  // Path doesn't match expected suffix pattern, return unchanged
  return p;
}
