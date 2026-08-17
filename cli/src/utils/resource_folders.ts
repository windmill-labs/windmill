/**
 * Utility module for handling resource folder naming conventions.
 *
 * This module centralizes the logic for detecting and manipulating paths
 * that contain resource folders (.flow, .app, .raw_app).
 *
 * The folder suffixes can be configured to use either dot-prefixed names
 * (.flow, .app, .raw_app) or dunder-prefixed names (__flow, __app, __raw_app).
 */

import { existsSync } from "node:fs";
import * as log from "../core/log.ts";
import { sep as SEP } from "node:path";
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
    log.debug("Using non-dotted paths (__flow, __app, __raw_app)");
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

/**
 * Check if a directory name uses the *wrong* folder suffix format for the
 * current nonDottedPaths setting. Returns the resource type if mismatched,
 * null if the name is fine (or not a resource folder at all).
 *
 * - nonDottedPaths=false (dotted mode): flags __flow, __app, __raw_app
 * - nonDottedPaths=true  (non-dotted):  flags .flow, .app, .raw_app
 */
export function hasWrongFormatSuffix(dirName: string): FolderResourceType | null {
  const wrongSuffixes = _nonDottedPaths ? DOTTED_SUFFIXES : NON_DOTTED_SUFFIXES;
  for (const [type, suffix] of Object.entries(wrongSuffixes)) {
    if (dirName.endsWith(suffix)) {
      return type as FolderResourceType;
    }
  }
  return null;
}

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
 * Check if a path is inside a folder-based resource, checking BOTH dotted (.flow, .app, .raw_app)
 * and non-dotted (__flow, __app, __raw_app) formats regardless of the global nonDottedPaths setting.
 * Use this instead of isFolderResourcePath when the config may not yet be loaded or when
 * you need to handle mixed-format workspaces (e.g. generate-metadata scanning all files).
 */
export function isFolderResourcePathAnyFormat(p: string): boolean {
  const n = normalizeSep(p);
  for (const suffixes of [DOTTED_SUFFIXES, NON_DOTTED_SUFFIXES]) {
    if (
      n.includes(suffixes.flow + "/") ||
      n.includes(suffixes.app + "/") ||
      n.includes(suffixes.raw_app + "/")
    ) {
      return true;
    }
  }
  return false;
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

/**
 * Check if a path ends with a specific app metadata file
 * (inside the folder, e.g., ".app/app.yaml" or "__app/app.yaml")
 */
export function isAppFolderMetadataFile(p: string): boolean {
  return (
    p.endsWith(getMetadataPathSuffix("app", "yaml")) ||
    p.endsWith(getMetadataPathSuffix("app", "json"))
  );
}

/**
 * Check if a path ends with a specific flow metadata file
 * (inside the folder, e.g., ".flow/flow.yaml" or "__flow/flow.yaml")
 */
export function isFlowFolderMetadataFile(p: string): boolean {
  return (
    p.endsWith(getMetadataPathSuffix("flow", "yaml")) ||
    p.endsWith(getMetadataPathSuffix("flow", "json"))
  );
}

// ============================================================================
// Script Module Path Functions
// ============================================================================

/**
 * The suffix used for script module folders.
 * Unlike flows/apps, modules always use `__mod` (never dotted `.mod`)
 * to avoid confusion with file extensions.
 */
const MODULE_SUFFIX = "__mod";
/** dbt scripts carry a whole dbt project, not helper code. The folder says so,
 *  and it is what a dbt developer points `--project-dir` at. */
export const DBT_MODULE_SUFFIX = "__dbt";
const MODULE_SUFFIXES = [MODULE_SUFFIX, DBT_MODULE_SUFFIX];

/** A dbt project's descriptor, inside the project it configures and OPTIONAL:
 *  an unmodified dbt project is already a complete Windmill script, and this
 *  file only appears when one needs something Windmill-specific (run arguments,
 *  a named warehouse, an engine pin). Its absence is an empty descriptor, never
 *  a missing script. */
export const DBT_DESCRIPTOR_NAME = "wm_dbt.yaml";

/** Where a dbt script's descriptor lives, given its base path. */
export function dbtDescriptorPath(scriptBasePath: string): string {
  return scriptBasePath + DBT_MODULE_SUFFIX + "/" + DBT_DESCRIPTOR_NAME;
}

/** Whether an error is a dbt descriptor that simply is not there. */
export function isMissingDbtDescriptor(filePath: string, e: unknown): boolean {
  if ((e as { code?: string })?.code !== "ENOENT") return false;
  const norm = filePath.replaceAll("\\", "/");
  if (!isDbtDescriptorPath(norm)) return false;
  // And the PROJECT is there. Absent both, this is not a descriptor-less
  // project but a path that does not exist — a typo, or a project someone
  // deleted — and treating it as an empty descriptor pushes `modules:
  // undefined` over a deployed bundle, dropping it while reporting success.
  return existsSync(
    norm.slice(0, -DBT_DESCRIPTOR_NAME.length) + "dbt_project.yml"
  );
}

/** Whether a path is a dbt descriptor, i.e. a dbt script's content file. */
export function isDbtDescriptorPath(p: string): boolean {
  const norm = normalizeSep(p);
  const base = getScriptBasePathFromModulePath(norm);
  return base !== undefined && norm === dbtDescriptorPath(base);
}

/**
 * Module folder suffix for a script: `__dbt` for a dbt project, `__mod`
 * otherwise.
 */
export function getModuleFolderSuffix(language?: string): string {
  return language === "dbt" ? DBT_MODULE_SUFFIX : MODULE_SUFFIX;
}

/**
 * Check if a path is inside a script module folder.
 * Matches patterns like: .../my_script__mod/... or .../my_project__dbt/...
 */
export function isScriptModulePath(p: string): boolean {
  const n = normalizeSep(p);
  return MODULE_SUFFIXES.some((suffix) => n.includes(suffix + "/"));
}

/** Per-file ceiling for a dbt project's bundle. Real dbt code is small (about
 *  500 bytes median, 1.9 KB at p90 measured across dbt_utils), so this only
 *  ever catches a committed dataset, which belongs in the warehouse rather than
 *  in every version of the script. */
export const MAX_MODULE_BYTES = 5 * 1024 * 1024;

/**
 * Whether a dbt project file is one the bundle carries.
 *
 * A dbt project's authored files are text. A binary one -- an image under
 * `docs/`, a `.DS_Store`, a parquet seed -- would be read as mojibake and, if
 * it carries a NUL, rejected by Postgres with an opaque `unsupported Unicode
 * escape sequence`. Binary is detected the way `git` does it, by a NUL in the
 * first 8000 bytes, rather than by extension, which `docs/` and stray dotfiles
 * do not follow.
 *
 * The push, the staleness hash and the sync diff all ask this same question: a
 * file one drops and another keeps is a change no push can ever resolve.
 */
export function isBundledModuleFile(fullPath: string): boolean {
  return moduleFileExclusion(fullPath) === undefined;
}

/**
 * WHY the bundle does not carry a file, when it does not.
 *
 * The two reasons are not interchangeable. `binary` is dbt's own leftovers and
 * stray archives: nothing to say, so sync hides them. `oversized` is a file the
 * project authored and dbt WOULD read — a large seed CSV — so it has to stay
 * visible in the diff, or the push that reports the actionable size error never
 * runs and the remote project is silently left incomplete.
 */
export function moduleFileExclusion(
  fullPath: string,
): "binary" | "oversized" | undefined {
  // Size from `stat` and only the first 8 KB read: a project may sit next to a
  // multi-gigabyte parquet seed or a stray archive, and reading one whole just
  // to classify it would stall the sync or exhaust the CLI.
  let size: number;
  let fd: number;
  try {
    size = fs.statSync(fullPath).size;
    fd = fs.openSync(fullPath, "r");
  } catch {
    // Unreadable is not the same as excluded. A pull asks this about files that
    // do not exist locally yet, and answering "not carried" there would make
    // sync ignore the whole incoming project and write nothing.
    return undefined;
  }
  let binary: boolean;
  try {
    const head = Buffer.alloc(8000);
    const read = fs.readSync(fd, head, 0, 8000, 0);
    binary = head.subarray(0, read).includes(0);
  } catch {
    return undefined;
  } finally {
    fs.closeSync(fd);
  }
  if (binary) return "binary";
  return size > MAX_MODULE_BYTES ? "oversized" : undefined;
}

/**
 * The refusal an oversized dbt project file earns, raised WITHOUT reading it.
 *
 * dbt would have read the file, so deploying the project without it ships
 * something that compiles here and fails at run time with a missing relation —
 * hence an error rather than a skip. Every path that would otherwise load the
 * body (the sync map, the push) asks first: a multi-gigabyte seed must not be
 * buffered just to be refused.
 */
export function oversizedModuleFileError(relPath: string, size: number): Error {
  return new Error(
    `${relPath} is ${Math.ceil(size / 1024 / 1024)} MB, over the ` +
      `${MAX_MODULE_BYTES / 1024 / 1024} MB per-file limit for a dbt project file. ` +
      `Deploying without it would leave the project incomplete — shrink the file, or ` +
      `keep it out of the project folder.`,
  );
}

/**
 * Refuse an oversized dbt project file before its content is read. `undefined`
 * for everything else, including binary files the bundle merely drops.
 */
export function oversizedDbtFileError(
  fullPath: string,
  relPath: string,
): Error | undefined {
  if (!isDbtModulePath(relPath)) return undefined;
  if (moduleFileExclusion(fullPath) !== "oversized") return undefined;
  let size = 0;
  try {
    size = fs.statSync(fullPath).size;
  } catch {
    return undefined;
  }
  return oversizedModuleFileError(relPath, size);
}

/** Whether a path is inside a dbt project's module folder specifically: those
 *  files are taken verbatim, with no language inference. */
export function isDbtModulePath(p: string): boolean {
  // The OUTERMOST boundary decides, like `getScriptBasePathFromModulePath`.
  // Scanning anywhere in the path would call `foo__mod/vendor/x__dbt/a.ts` a dbt
  // project file, and the push would then look for `foo.script.yaml` instead of
  // the ordinary module entry point and skip the edit.
  const norm = normalizeSep(p);
  const base = getScriptBasePathFromModulePath(norm);
  return base !== undefined && norm.startsWith(base + DBT_MODULE_SUFFIX + "/");
}

/** dbt writes these; a project authors them nowhere. Importing a stale
 *  `target/` would ship a manifest this runtime then reads as the graph, and
 *  `dbt_packages/` is a vendored copy the worker restores from its own cache. */
const DBT_GENERATED_DIRS = ["target", "dbt_packages", "logs", ".git", ".venv", "__pycache__"];

const dbtGeneratedDirsCache = new Map<
  string,
  { stamp: string; dirs: Set<string> }
>();

/** `{{ env_var('NAME') }}` / `{{ env_var("NAME", "default") }}`. */
const DBT_ENV_VAR_CALL =
  /\{\{\s*env_var\(\s*['"]([^'"]+)['"]\s*(?:,\s*['"]([^'"]*)['"]\s*)?\)\s*\}\}/g;

/**
 * Render `dbt_project.yml`'s own `env_var()` calls, which dbt allows there too.
 * A directory setting left as its template names no directory on disk, so the
 * generated tree it points at would be bundled as project source.
 *
 * Against `process.env`, because the CLI runs where the project was built: that
 * is the environment dbt used to produce the tree being read.
 */
export function renderDbtEnvVars(value: string): string {
  return value.replace(
    DBT_ENV_VAR_CALL,
    (whole, name: string, fallback: string | undefined) =>
      process.env[name] ?? fallback ?? whole,
  );
}

/**
 * Files that keep a project's secrets next to it rather than in it: dbt reads
 * none of them (`env_var()` takes the process environment), and the documented
 * way into a bundle is `cp -r my-project/.`, which copies whatever the checkout
 * holds — including the `.env` a `.gitignore` was keeping out of the repo.
 */
export function isLocalSecretFile(name: string): boolean {
  return name === ".env" || name.startsWith(".env.") || name === ".envrc";
}

/**
 * Directories to leave out of a dbt project's module bundle, as project-relative
 * paths — `target-path` and friends may be nested (`build/target`).
 *
 * `target-path`, `packages-install-path` and `clean-targets` are configurable,
 * so they are read from the project rather than assumed. Cached per project
 * folder: this is called once per file of a sync.
 */
export function dbtGeneratedDirs(moduleFolderPath: string): Set<string> {
  const projectFile = path.join(moduleFolderPath, "dbt_project.yml");
  // Cached against the project file's identity, not merely its folder: `wmill
  // dev` is a long-running process, so a `target-path` edited mid-session would
  // otherwise keep excluding the old directory and start bundling the new one
  // as project source. One entry per folder, replaced when the file changes.
  let stamp = "";
  try {
    const st = fs.statSync(projectFile);
    stamp = `${st.mtimeMs}:${st.size}`;
  } catch {
    // No project file yet: the defaults apply, and "absent" is its own stamp.
  }
  const cached = dbtGeneratedDirsCache.get(moduleFolderPath);
  if (cached && cached.stamp === stamp) return cached.dirs;
  const dirs = new Set<string>(DBT_GENERATED_DIRS);
  const add = (raw: string) => {
    const v = normalizeSep(renderDbtEnvVars(raw).trim().replace(/^["']|["']$/g, ""))
      .replace(/^\.\//, "")
      .replace(/\/+$/, "");
    // A configured path that escapes the project is dbt's problem, not ours;
    // ignoring it here just means those files stay in the bundle.
    if (v && !v.startsWith("/") && !v.split("/").includes("..")) dirs.add(v);
  };
  try {
    const projectYml = fs.readFileSync(projectFile, "utf-8");
    for (const m of projectYml.matchAll(
      /^\s*(?:target-path|packages-install-path)\s*:\s*([^\n#]+)/gm,
    )) {
      add(m[1]);
    }
    // `clean-targets` in either of dbt's two spellings: inline `[a, b]`, and the
    // block form, whose entries are on the lines that follow.
    const lines = projectYml.split(/\r?\n/);
    for (let i = 0; i < lines.length; i++) {
      const head = lines[i].match(/^\s*clean-targets\s*:\s*(.*)$/);
      if (!head) continue;
      const inline = head[1].match(/^\[([^\]]*)\]/);
      if (inline) {
        inline[1].split(",").forEach(add);
        continue;
      }
      for (let j = i + 1; j < lines.length; j++) {
        const item = lines[j].match(/^\s+-\s*([^\n#]+)$/);
        if (!item) break;
        add(item[1]);
      }
    }
  } catch {
    // No dbt_project.yml yet (a descriptor pushed before its project): the
    // defaults still apply.
  }
  dbtGeneratedDirsCache.set(moduleFolderPath, { stamp, dirs });
  return dirs;
}

/**
 * Whether a project-relative path sits inside one of `dirs`. Compared segment
 * by segment: `targetx/a` must not match a configured `target`.
 */
export function isUnderGeneratedDir(rel: string, dirs: Set<string>): boolean {
  const n = normalizeSep(rel);
  for (const d of dirs) {
    if (n === d || n.startsWith(d + "/")) return true;
  }
  return false;
}

/**
 * Whether a path under a `__dbt/` folder is one dbt generated rather than one
 * the project authors. Those never belong to the bundle, so sync must not offer
 * them as items of their own either.
 */
export function isDbtGeneratedPath(p: string): boolean {
  const n = normalizeSep(p);
  // Anchored on the outermost boundary, like every other helper here. Matching
  // `__dbt/` anywhere would call `foo__mod/vendor/x__dbt/target/a.ts` generated
  // dbt output, and `ignoreF` would then exclude an ordinary module file so a
  // module-only edit never deploys its parent script.
  if (!isDbtModulePath(n)) return false;
  const base = getScriptBasePathFromModulePath(n)!;
  const projectRoot = base + DBT_MODULE_SUFFIX;
  const rel = n.slice(projectRoot.length + 1);
  if (isUnderGeneratedDir(rel, dbtGeneratedDirs(projectRoot))) {
    return true;
  }
  // Not generated, but not carried either: the bundle drops it, so the diff
  // must not keep offering it as a pending change. An OVERSIZED one is the
  // exception — see `moduleFileExclusion`: hiding it here is what would make an
  // edit to a large seed report no change at all.
  return (
    isLocalSecretFile(n.slice(n.lastIndexOf("/") + 1)) ||
    moduleFileExclusion(p) === "binary"
  );
}

/**
 * Build the module folder path from a script's base path (without extension).
 * e.g., "f/my_script" -> "f/my_script__mod", or "__dbt" for a dbt project.
 */
export function buildModuleFolderPath(scriptBasePath: string, language?: string): string {
  return scriptBasePath + getModuleFolderSuffix(language);
}

/**
 * Check if a file inside a __mod/ folder is the main entry point (script.{ext}).
 * Entry points are files named "script.*" directly under __mod/ (not in subdirs).
 */
export function isModuleEntryPoint(p: string): boolean {
  const norm = normalizeSep(p);
  // Anchored on the OUTERMOST module boundary, like
  // `getScriptBasePathFromModulePath`. Scanning for `__mod/` alone would match a
  // `legacy__mod/` directory nested inside a dbt project — dbt owns those names
  // verbatim — and call its `script.ts` this script's entry point.
  const base = getScriptBasePathFromModulePath(norm);
  if (base === undefined) return false;
  // A dbt project's entry point is its descriptor, which sits INSIDE the
  // project so that an author writes nothing outside the directory dbt itself
  // reads.
  if (norm.startsWith(base + DBT_MODULE_SUFFIX + "/")) {
    return norm === dbtDescriptorPath(base);
  }
  const suffix = MODULE_SUFFIX + "/";
  if (!norm.startsWith(base + suffix)) return false;
  const rest = norm.slice(base.length + suffix.length);
  return rest.startsWith("script.") && !rest.includes("/");
}

/**
 * Extract the script base path from a module folder entry.
 * e.g., "u/admin/my_script__mod/script.ts" -> "u/admin/my_script"
 * e.g., "u/admin/my_script__mod/helper.ts" -> "u/admin/my_script"
 * e.g., "f/x/proj__dbt/models/a.sql" -> "f/x/proj"
 */
export function getScriptBasePathFromModulePath(p: string): string | undefined {
  const norm = normalizeSep(p);
  // The OUTERMOST boundary, not the first suffix that happens to match. A dbt
  // project's directories are the author's verbatim, so `foo__dbt/models/
  // legacy__mod/a.sql` is legal — taking `__mod` first would call
  // `foo__dbt/models/legacy` the script and look for a descriptor that is not
  // there, silently skipping the deploy.
  let best: number | undefined;
  for (const suffix of MODULE_SUFFIXES) {
    const idx = norm.indexOf(suffix + "/");
    if (idx !== -1 && (best === undefined || idx < best)) best = idx;
  }
  return best === undefined ? undefined : norm.slice(0, best);
}

/**
 * Convert a local script file path to its Windmill API remote path.
 * Handles both folder layout (`u/admin/my_script__mod/script.ts` -> `u/admin/my_script`)
 * and flat layout (`u/admin/my_script.ts` -> `u/admin/my_script`).
 *
 * For flat layout, splits at the FIRST `.` which means a script under a
 * folder containing a `.` in its name (e.g. `u/my.folder/script.ts`) gets
 * truncated; that pre-dates this helper and is preserved here for parity.
 */
export function scriptPathToRemotePath(p: string): string {
  return (
    isModuleEntryPoint(p)
      ? getScriptBasePathFromModulePath(p)!
      : p.substring(0, p.indexOf("."))
  ).replaceAll(SEP, "/");
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
