/**
 * Utility module for handling resource folder naming conventions.
 *
 * This module centralizes the logic for detecting and manipulating paths
 * that contain resource folders (.flow, .app, .raw_app).
 *
 * The folder suffixes can be configured to use either dot-prefixed names
 * (.flow, .app, .raw_app) or dunder-prefixed names (__flow, __app, __raw_app).
 */

import { SEP } from "../../deps.ts";

// Resource types that use folder-based storage
export type FolderResourceType = "flow" | "app" | "raw_app";

// Configuration for folder suffixes - can be switched between dot and dunder prefixes
// The default uses dot-prefixed names (.flow, .app, .raw_app)
const FOLDER_SUFFIXES = {
  flow: ".flow",
  app: ".app",
  raw_app: ".raw_app",
} as const;

// Metadata file names inside each folder type
const METADATA_FILES = {
  flow: { yaml: "flow.yaml", json: "flow.json" },
  app: { yaml: "app.yaml", json: "app.json" },
  raw_app: { yaml: "raw_app.yaml", json: "raw_app.json" },
} as const;

/**
 * Get the folder suffix for a resource type (e.g., ".flow", ".app", ".raw_app")
 */
export function getFolderSuffix(type: FolderResourceType): string {
  return FOLDER_SUFFIXES[type];
}

/**
 * Get the folder suffix with path separator (e.g., ".flow/", ".app/", ".raw_app/")
 */
export function getFolderSuffixWithSep(type: FolderResourceType): string {
  return FOLDER_SUFFIXES[type] + SEP;
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
 * Get the full metadata file path suffix (e.g., ".flow/flow.yaml")
 */
export function getMetadataPathSuffix(
  type: FolderResourceType,
  format: "yaml" | "json"
): string {
  return FOLDER_SUFFIXES[type] + "/" + METADATA_FILES[type][format];
}

// ============================================================================
// Path Detection Functions
// ============================================================================

/**
 * Check if a path is inside a flow folder
 */
export function isFlowPath(p: string): boolean {
  return p.includes(FOLDER_SUFFIXES.flow + SEP);
}

/**
 * Check if a path is inside an app folder
 */
export function isAppPath(p: string): boolean {
  return p.includes(FOLDER_SUFFIXES.app + SEP);
}

/**
 * Check if a path is inside a raw_app folder
 */
export function isRawAppPath(p: string): boolean {
  return p.includes(FOLDER_SUFFIXES.raw_app + SEP);
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
export function detectFolderResourceType(
  p: string
): FolderResourceType | null {
  if (isFlowPath(p)) return "flow";
  if (isAppPath(p)) return "app";
  if (isRawAppPath(p)) return "raw_app";
  return null;
}

/**
 * Check if a path is inside a raw app backend folder.
 * Matches patterns like: .../myApp.raw_app/backend/...
 */
export function isRawAppBackendPath(filePath: string): boolean {
  // Normalize path separators for consistent matching
  const normalizedPath = filePath.replaceAll(SEP, "/");
  // Check if path contains pattern: *.raw_app/backend/
  const pattern = new RegExp(
    `\\${FOLDER_SUFFIXES.raw_app.replace(".", "\\.")}/backend/`
  );
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
  const suffix = FOLDER_SUFFIXES[type] + SEP;
  const index = p.indexOf(suffix);
  if (index === -1) return null;
  return p.substring(0, index);
}

/**
 * Extract the folder path (resource name + folder suffix)
 * e.g., "f/my_flow.flow/flow.yaml" -> "f/my_flow.flow/"
 */
export function extractFolderPath(
  p: string,
  type: FolderResourceType
): string | null {
  const suffix = FOLDER_SUFFIXES[type] + SEP;
  const index = p.indexOf(suffix);
  if (index === -1) return null;
  return p.substring(0, index) + suffix;
}

/**
 * Build a folder path from a resource name
 * e.g., ("f/my_flow", "flow") -> "f/my_flow.flow"
 */
export function buildFolderPath(
  resourceName: string,
  type: FolderResourceType
): string {
  return resourceName + FOLDER_SUFFIXES[type];
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
  return resourceName + FOLDER_SUFFIXES[type] + SEP + METADATA_FILES[type][format];
}

// ============================================================================
// Folder Validation Functions
// ============================================================================

/**
 * Check if a directory name ends with a specific resource folder suffix
 * e.g., "my_app.raw_app" ends with ".raw_app"
 */
export function hasFolderSuffix(
  dirName: string,
  type: FolderResourceType
): boolean {
  return dirName.endsWith(FOLDER_SUFFIXES[type]);
}

/**
 * Validate that a directory name has the expected folder suffix
 * Returns an error message if invalid, null if valid
 */
export function validateFolderName(
  dirName: string,
  type: FolderResourceType
): string | null {
  if (!hasFolderSuffix(dirName, type)) {
    return `'${dirName}' does not end with '${FOLDER_SUFFIXES[type]}'`;
  }
  return null;
}

/**
 * Extract the resource name from a folder name by removing the suffix
 * e.g., "my_app.raw_app" -> "my_app"
 */
export function extractNameFromFolder(
  folderName: string,
  type: FolderResourceType
): string {
  const suffix = FOLDER_SUFFIXES[type];
  if (folderName.endsWith(suffix)) {
    return folderName.substring(0, folderName.length - suffix.length);
  }
  return folderName;
}

// ============================================================================
// Metadata File Detection Functions
// ============================================================================

/**
 * Check if a path ends with a flow metadata file suffix
 */
export function isFlowMetadataFile(p: string): boolean {
  return p.endsWith(".flow.json") || p.endsWith(".flow.yaml");
}

/**
 * Check if a path ends with an app metadata file suffix
 */
export function isAppMetadataFile(p: string): boolean {
  return p.endsWith(".app.json") || p.endsWith(".app.yaml");
}

/**
 * Check if a path ends with a raw_app metadata file suffix
 */
export function isRawAppMetadataFile(p: string): boolean {
  return p.endsWith(".raw_app.json") || p.endsWith(".raw_app.yaml");
}

/**
 * Check if a path ends with a specific raw_app metadata file
 * (inside the folder, e.g., ".raw_app/raw_app.yaml")
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
  return FOLDER_SUFFIXES[type] + "/" + METADATA_FILES[type][format];
}

/**
 * Transform a JSON path to the appropriate directory/file path for sync
 * e.g., "f/my_flow.flow.json" -> "f/my_flow.flow"
 */
export function transformJsonPathToDir(
  p: string,
  type: FolderResourceType
): string {
  const suffix = METADATA_FILES[type].json;
  return p.replace(suffix, type);
}
