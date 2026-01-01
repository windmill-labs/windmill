// deno-lint-ignore-file no-explicit-any
import { GlobalOptions } from "../types.ts";
import { SEP, colors, log, yamlParseFile, yamlStringify } from "../../deps.ts";
import {
  ScriptMetadata,
  defaultScriptMetadata,
} from "../../bootstrap/script_bootstrap.ts";
import { Workspace } from "../commands/workspace/workspace.ts";
import {
  ScriptLanguage,
  workspaceDependenciesLanguages,
} from "./script_common.ts";
import { inferContentTypeFromFilePath } from "./script_common.ts";
import { findCodebase, yamlOptions } from "../commands/sync/sync.ts";
import { generateHash, readInlinePathSync, getHeaders } from "./utils.ts";

import { SyncCodebase } from "./codebase.ts";
import { argSigToJsonSchemaType } from "../../windmill-utils-internal/src/parse/parse-schema.ts";
import { getIsWin } from "./utils.ts";

export class LockfileGenerationError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "LockfileGenerationError";
  }
}

export async function generateAllMetadata() {}

// =============================================================================
// CONSTANTS - Import Patterns and File Extensions
// =============================================================================

const TS_IMPORT_PATTERNS = {
  ES6_IMPORT: /import\s+(?:(?:\*\s+as\s+\w+)|(?:\{[^}]*\})|(?:\w+))\s+from\s+['"]([^'"]+)['"]/g,
  DYNAMIC_IMPORT: /import\s*\(\s*['"]([^'"]+)['"]\s*\)/g,
  REQUIRE: /require\s*\(\s*['"]([^'"]+)['"]\s*\)/g,
} as const;

const PYTHON_IMPORT_PATTERNS = {
  RELATIVE: /from\s+(\.+\w*(?:\.\w+)*)\s+import/g,
  ABSOLUTE: /from\s+([fu]\.\w+(?:\.\w+)*)\s+import/g,
} as const;

const TS_EXTENSION_PATTERN = /\.ts$/;
const PATH_SEPARATOR = '/';

const LANGUAGE_EXTENSIONS: Record<string, string[]> = {
  python3: ['.py'],
  bun: ['.ts', '.js'],
  deno: ['.ts', '.js'],
  nativets: ['.ts', '.js'],
};

// =============================================================================
// UTILITY FUNCTIONS - Path Operations
// =============================================================================

/**
 * Extract directory path from a script path
 */
function getScriptDirectory(scriptPath: string): string {
  const lastSlashIndex = scriptPath.lastIndexOf(PATH_SEPARATOR);
  return lastSlashIndex >= 0 ? scriptPath.substring(0, lastSlashIndex) : '';
}

/**
 * Convert file path to remote path (remove extension, normalize separators)
 */
function toRemotePath(scriptPath: string): string {
  const lastDotIndex = scriptPath.lastIndexOf('.');
  const pathWithoutExtension = lastDotIndex >= 0
    ? scriptPath.substring(0, lastDotIndex)
    : scriptPath;
  return pathWithoutExtension.replaceAll(SEP, PATH_SEPARATOR);
}

/**
 * Resolve a relative import path against a base script path
 */
function resolveImportPath(
  importPath: string,
  scriptPath: string
): string | null {
  if (importPath.startsWith(PATH_SEPARATOR)) {
    return importPath.substring(1); // Absolute import - strip leading slash
  } else if (importPath.startsWith('.')) {
    const scriptDir = getScriptDirectory(scriptPath);
    return normalizePath(`${scriptDir}${PATH_SEPARATOR}${importPath}`);
  }
  return null; // Not a local import
}

/**
 * Get file extensions for a given language
 */
function getExtensionsForLanguage(language: ScriptLanguage): string[] {
  return LANGUAGE_EXTENSIONS[language] ?? ['.ts', '.js'];
}

/**
 * Normalize a path by resolving .. and . segments
 */
function normalizePath(path: string): string | null {
  const parts = path.split(PATH_SEPARATOR);
  const result: string[] = [];

  for (const part of parts) {
    if (part === '' || part === '.') {
      continue;
    } else if (part === '..') {
      if (result.length === 0) {
        return null; // Invalid path going above root
      }
      result.pop();
    } else {
      result.push(part);
    }
  }

  return result.join(PATH_SEPARATOR);
}

// =============================================================================
// RAW WORKSPACE DEPENDENCIES
// =============================================================================

export async function getRawWorkspaceDependencies(): Promise<Record<string, string>> {
  const rawWorkspaceDeps: Record<string, string> = {};

  try {
    for await (const entry of Deno.readDir("dependencies")) {
      if (entry.isDirectory) continue;

      const filePath = `dependencies/${entry.name}`;
      const content = await Deno.readTextFile(filePath);

      // Find matching language
      for (const lang of workspaceDependenciesLanguages) {
        if (entry.name.endsWith(lang.filename)) {
          // Check if out of sync
          const contentHash = await generateHash(content + filePath);
          const isUpToDate = await checkifMetadataUptodate(filePath, contentHash, undefined);

          if (!isUpToDate) {
            rawWorkspaceDeps[filePath] = content;
          }
          break;
        }
      }
    }
  } catch {
    // dependencies directory doesn't exist
  }
  return rawWorkspaceDeps;
}

// =============================================================================
// IMPORT EXTRACTION FUNCTIONS
// =============================================================================

/**
 * Extract relative imports from TypeScript/JavaScript code
 * Matches imports like:
 * - import { foo } from "./bar"
 * - import { foo } from "../bar"
 * - import { foo } from "/f/folder/script"
 */
function extractTSRelativeImports(code: string, scriptPath: string): string[] {
  const imports: string[] = [];

  const patterns = [
    TS_IMPORT_PATTERNS.ES6_IMPORT,
    TS_IMPORT_PATTERNS.DYNAMIC_IMPORT,
    TS_IMPORT_PATTERNS.REQUIRE
  ];

  for (const pattern of patterns) {
    for (const match of code.matchAll(pattern)) {
      const importPath = match[1];
      if (!importPath) continue; // Type guard

      const cleanPath = importPath.replace(TS_EXTENSION_PATTERN, '');
      const resolved = resolveImportPath(cleanPath, scriptPath);

      if (resolved) {
        imports.push(resolved);
      }
    }
  }

  return imports;
}

/**
 * Parse Python relative import and calculate target path
 */
function parsePythonRelativeImport(
  importMatch: string,
  scriptPath: string
): string | null {
  const leadingDotsMatch = importMatch.match(/^\.+/);
  const leadingDots = leadingDotsMatch?.[0] ?? '';
  const module = importMatch.substring(leadingDots.length);

  // Calculate how many levels to go up
  const levelsUp = Math.max(0, leadingDots.length - 1);

  const scriptDir = getScriptDirectory(scriptPath);
  const pathParts = scriptDir.split(PATH_SEPARATOR).filter(p => p.length > 0);

  // Go up the specified number of levels
  const targetParts = pathParts.slice(0, Math.max(0, pathParts.length - levelsUp));

  // Add the module path
  if (module) {
    targetParts.push(...module.split('.'));
  }

  return targetParts.length > 0 ? targetParts.join(PATH_SEPARATOR) : null;
}

/**
 * Parse Python absolute import (f.* or u.*)
 */
function parsePythonAbsoluteImport(importMatch: string): string {
  return importMatch.replace(/\./g, PATH_SEPARATOR);
}

/**
 * Extract relative imports from Python code
 * Matches imports like:
 * - from .module import something
 * - from ..module import something
 * - from f.folder.script import something
 * - from u.folder.script import something
 */
function extractPythonRelativeImports(code: string, scriptPath: string): string[] {
  const imports: string[] = [];

  // Process relative imports
  for (const match of code.matchAll(PYTHON_IMPORT_PATTERNS.RELATIVE)) {
    const resolved = parsePythonRelativeImport(match[1], scriptPath);
    if (resolved) {
      imports.push(resolved);
    }
  }

  // Process absolute folder imports
  for (const match of code.matchAll(PYTHON_IMPORT_PATTERNS.ABSOLUTE)) {
    imports.push(parsePythonAbsoluteImport(match[1]));
  }

  return imports;
}

// =============================================================================
// SCRIPT COLLECTION HELPERS
// =============================================================================

/**
 * Try to read an imported script file with multiple extension attempts
 */
async function tryReadImportedScript(
  importPath: string,
  extensions: string[]
): Promise<{ content: string; filePath: string; language: ScriptLanguage } | null> {
  const basePath = importPath.replaceAll(PATH_SEPARATOR, SEP);

  for (const ext of extensions) {
    const filePath = basePath + ext;

    try {
      const content = await Deno.readTextFile(filePath);
      const language = inferContentTypeFromFilePath(filePath, 'bun');
      return { content, filePath, language };
    } catch (e) {
      if (!(e instanceof Deno.errors.NotFound)) {
        // Log unexpected errors (permissions, etc.)
        log.debug(`Error reading ${filePath}: ${e}`);
      }
      continue;
    }
  }

  log.debug(`Could not find import '${importPath}' with extensions [${extensions.join(', ')}]`);
  return null;
}

/**
 * Process a single import and collect its dependencies recursively
 */
async function processImport(
  importPath: string,
  language: ScriptLanguage,
  visited: Set<string>,
  localScripts: Record<string, { content: string; language: ScriptLanguage }>
): Promise<void> {
  const extensions = getExtensionsForLanguage(language);

  const scriptData = await tryReadImportedScript(importPath, extensions);
  if (!scriptData) {
    return; // File not found, already logged
  }

  // Store the imported script
  localScripts[importPath] = {
    content: scriptData.content,
    language: scriptData.language
  };

  // Recursively collect imports from this script
  const nestedScripts = await collectLocalScripts(
    scriptData.filePath,
    scriptData.language,
    visited
  );

  // Merge nested scripts (avoid overwriting)
  for (const [path, data] of Object.entries(nestedScripts)) {
    localScripts[path] ??= data; // Use nullish coalescing assignment
  }
}

/**
 * Extract imports based on script language
 */
function extractImportsForLanguage(
  content: string,
  remotePath: string,
  language: ScriptLanguage
): string[] {
  if (language === 'bun' || language === 'deno' || language === 'nativets') {
    return extractTSRelativeImports(content, remotePath);
  } else if (language === 'python3') {
    return extractPythonRelativeImports(content, remotePath);
  }
  return [];
}

/**
 * Collect local scripts that are imported by the given script
 * This function recursively traverses imports to find all dependencies
 *
 * @param scriptPath The path to the script file to analyze
 * @param language The language of the script
 * @param visited Set of already visited script paths to avoid cycles
 * @returns Record of script path -> script content for all imported local scripts
 */
export async function collectLocalScripts(
  scriptPath: string,
  language: ScriptLanguage,
  visited: Set<string> = new Set()
): Promise<Record<string, { content: string; language: ScriptLanguage }>> {
  const localScripts: Record<string, { content: string; language: ScriptLanguage }> = {};

  const remotePath = toRemotePath(scriptPath);

  if (visited.has(remotePath)) {
    return localScripts;
  }
  visited.add(remotePath);

  let scriptContent: string;
  try {
    scriptContent = await Deno.readTextFile(scriptPath);
  } catch (e) {
    log.debug(`Could not read script file ${scriptPath}: ${e}`);
    return localScripts;
  }

  // Extract imports based on language
  const imports = extractImportsForLanguage(scriptContent, remotePath, language);

  // Process each import
  for (const importPath of imports) {
    await processImport(importPath, language, visited, localScripts);
  }

  return localScripts;
}

export function workspaceDependenciesPathToLanguageAndFilename(path: string): { name: string | undefined, language: ScriptLanguage } | undefined {
    const relativePath = path.replace("dependencies/", "");
    for (const { filename, language } of workspaceDependenciesLanguages) {
      if (relativePath.endsWith(filename)) {
        return {
          name: relativePath === filename ? undefined : relativePath.replace("." + filename, ""),
          language
        };
      }
    }
}

// on windows, when using powershell, blue is not readable
export async function blueColor(): Promise<(x: string) => void> {
  const isWin = await getIsWin();
  return isWin ? colors.black : colors.blue;
}

export async function generateScriptMetadataInternal(
  scriptPath: string,
  workspace: Workspace,
  opts: GlobalOptions & {
    lockOnly?: boolean | undefined;
    schemaOnly?: boolean | undefined;
    defaultTs?: "bun" | "deno";
  },
  dryRun: boolean,
  noStaleMessage: boolean,
  rawWorkspaceDependencies: Record<string, string>,
  codebases: SyncCodebase[],
  justUpdateMetadataLock?: boolean
): Promise<string | undefined> {
  const remotePath = scriptPath
    .substring(0, scriptPath.indexOf("."))
    .replaceAll(SEP, "/");

  const language = inferContentTypeFromFilePath(scriptPath, opts.defaultTs);

  // Filter workspace dependencies to only include those matching the script's language
  const filteredRawWorkspaceDependencies: Record<string, string> = {};
  for (const [depPath, depContent] of Object.entries(rawWorkspaceDependencies)) {
    const depInfo = workspaceDependenciesPathToLanguageAndFilename(depPath);
    if (depInfo && depInfo.language === language) {
      filteredRawWorkspaceDependencies[depPath] = depContent;
    }
  }

  const metadataWithType = await parseMetadataFile(
    remotePath,
    undefined,
  );

  // read script content
  const scriptContent = await Deno.readTextFile(scriptPath);
  const metadataContent = await Deno.readTextFile(metadataWithType.path);
  
  // Note: rawWorkspaceDependencies are now passed in as parameter instead of being searched hierarchically
  let hash = await generateScriptHash(filteredRawWorkspaceDependencies, scriptContent, metadataContent);

  if (await checkifMetadataUptodate(remotePath, hash, undefined)) {
    if (!noStaleMessage) {
      log.info(
        colors.green(`Script ${remotePath} metadata is up-to-date, skipping`)
      );
    }
    return;
  } else if (dryRun) {
    return `${remotePath} (${language})`;
  }

  if (!justUpdateMetadataLock) {
    log.info(colors.gray(`Generating metadata for ${scriptPath}`));
  }

  const metadataParsedContent = metadataWithType?.payload as Record<
    string,
    any
  >;

  if (!opts.lockOnly && !justUpdateMetadataLock) {
    await updateScriptSchema(
      scriptContent,
      language,
      metadataParsedContent,
      scriptPath
    );
  }

  if (!opts.schemaOnly && !justUpdateMetadataLock) {
    const hasCodebase = findCodebase(scriptPath, codebases) != undefined;

    if (!hasCodebase) {
      await updateScriptLock(
        workspace,
        scriptContent,
        language,
        remotePath,
        metadataParsedContent,
        filteredRawWorkspaceDependencies,
        scriptPath
      );
    } else {
      metadataParsedContent.lock = "";
    }
  } else {
    metadataParsedContent.lock =
      "!inline " + remotePath.replaceAll(SEP, "/") + ".script.lock";
  }
  let metaPath = remotePath + ".script.yaml";
  let newMetadataContent = yamlStringify(metadataParsedContent, yamlOptions);
  if (metadataWithType.isJson) {
    metaPath = remotePath + ".script.json";
    newMetadataContent = JSON.stringify(metadataParsedContent);
  }

  const metadataContentUsedForHash = newMetadataContent;

  hash = await generateScriptHash(
    filteredRawWorkspaceDependencies,
    scriptContent,
    metadataContentUsedForHash
  );
  await updateMetadataGlobalLock(remotePath, hash);
  if (!justUpdateMetadataLock) {
    await Deno.writeTextFile(metaPath, newMetadataContent);
  }
  return `${remotePath} (${language})`;
}

export async function updateScriptSchema(
  scriptContent: string,
  language: ScriptLanguage,
  metadataContent: Record<string, any>,
  path: string
): Promise<void> {
  // infer schema from script content and update it inplace
  const result = await inferSchema(
    language,
    scriptContent,
    metadataContent.schema,
    path
  );
  metadataContent.schema = result.schema;
  if (result.has_preprocessor) {
    metadataContent.has_preprocessor = result.has_preprocessor;
  } else {
    delete metadataContent.has_preprocessor;
  }
  if (result.no_main_func) {
    metadataContent.no_main_func = result.no_main_func;
  } else {
    delete metadataContent.no_main_func;
  }
}

// =============================================================================
// LOCKFILE GENERATION HELPERS
// =============================================================================

/**
 * Raw script entry for API payload
 */
interface RawScriptEntry {
  raw_code: string;
  language: ScriptLanguage;
  script_path: string;
}

/**
 * Build the raw_scripts payload for lockfile generation
 */
function buildRawScriptsPayload(
  mainScript: { content: string; language: ScriptLanguage; path: string },
  localScripts: Record<string, { content: string; language: ScriptLanguage }>
): RawScriptEntry[] {
  return [
    {
      raw_code: mainScript.content,
      language: mainScript.language,
      script_path: mainScript.path,
    },
    ...Object.entries(localScripts).map(([importPath, scriptData]) => ({
      raw_code: scriptData.content,
      language: scriptData.language,
      script_path: importPath,
    }))
  ];
}

async function updateScriptLock(
  workspace: Workspace,
  scriptContent: string,
  language: ScriptLanguage,
  remotePath: string,
  metadataContent: Record<string, any>,
  rawWorkspaceDependencies: Record<string, string>,
  scriptPath: string
): Promise<void> {
  if (
    !(
      workspaceDependenciesLanguages.some((l) => l.language == language) ||
      language == "deno" ||
      language == "rust" ||
      language == "ansible"
    )
  ) {
    return;
  }

  // Collect local scripts that are imported by this script
  const localScripts = await collectLocalScripts(scriptPath, language);
  const localScriptCount = Object.keys(localScripts).length;

  // Log workspace dependencies and local scripts
  if (Object.keys(rawWorkspaceDependencies).length > 0) {
    const dependencyPaths = Object.keys(rawWorkspaceDependencies).join(', ');
    log.info(`Generating script lock for ${remotePath} with raw workspace dependencies: ${dependencyPaths}`);
  }

  if (localScriptCount > 0) {
    const scriptPaths = Object.keys(localScripts).join(', ');
    log.info(`Found ${localScriptCount} local script(s) imported by ${remotePath}: ${scriptPaths}`);
  }

  // Build raw_scripts payload with main script and all its local imports
  const rawScripts = buildRawScriptsPayload(
    { content: scriptContent, language, path: remotePath },
    localScripts
  );

  // generate the script lock running a dependency job in Windmill and update it inplace
  // TODO: update this once the client is released
  const extraHeaders = getHeaders();
  const rawResponse = await fetch(
    `${workspace.remote}api/w/${workspace.workspaceId}/jobs/run/dependencies`,
    {
      method: "POST",
      headers: {
        Cookie: `token=${workspace.token}`,
        "Content-Type": "application/json",
        ...extraHeaders,
      },
      body: JSON.stringify({
        raw_scripts: rawScripts,
        raw_workspace_dependencies: Object.keys(rawWorkspaceDependencies).length > 0
          ? rawWorkspaceDependencies : null,
        entrypoint: remotePath,
      }),
    }
  );

  let responseText = "reading response failed";
  try {
    responseText = await rawResponse.text();
    const response = JSON.parse(responseText);
    const lock = response.lock;
    if (lock === undefined) {
      if (response?.["error"]?.["message"]) {
        throw new LockfileGenerationError(
          `Failed to generate lockfile: ${response?.["error"]?.["message"]}`
        );
      }
      throw new LockfileGenerationError(
        `Failed to generate lockfile: ${JSON.stringify(response, null, 2)}`
      );
    }
    const lockPath = remotePath + ".script.lock";
    if (lock != "") {
      await Deno.writeTextFile(lockPath, lock);
      metadataContent.lock = "!inline " + lockPath.replaceAll(SEP, "/");
    } else {
      try {
        if (await Deno.stat(lockPath)) {
          await Deno.remove(lockPath);
        }
      } catch (e) {
        log.info(colors.yellow(`Error removing lock file ${lockPath}: ${e}`));
      }
      metadataContent.lock = "";
    }
  } catch (e) {
    if (e instanceof LockfileGenerationError) {
      throw e;
    }
    throw new LockfileGenerationError(
      `Failed to generate lockfile:${rawResponse.statusText}, ${responseText}, ${e}`
    );
  }
}

////////////////////////////////////////////////////////////////////////////////////////////
// below functions copied from Windmill's FE inferArgs function. TODO: refactor           //
////////////////////////////////////////////////////////////////////////////////////////////
export async function inferSchema(
  language: ScriptLanguage,
  content: string,
  currentSchema: any,
  path: string
): Promise<{
  schema: any;
  has_preprocessor: boolean | undefined;
  no_main_func: boolean | undefined;
}> {
  let inferedSchema: any;
  if (language === "python3") {
    const { parse_python } = await import(
      "../../wasm/py/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_python(content));
  } else if (language === "nativets") {
    const { parse_deno } = await import(
      "../../wasm/ts/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_deno(content));
  } else if (language === "bun") {
    const { parse_deno } = await import(
      "../../wasm/ts/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_deno(content));
  } else if (language === "deno") {
    const { parse_deno } = await import(
      "../../wasm/ts/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_deno(content));
  } else if (language === "go") {
    const { parse_go } = await import("../../wasm/go/windmill_parser_wasm.js");
    inferedSchema = JSON.parse(parse_go(content));
  } else if (language === "mysql") {
    const { parse_mysql } = await import(
      "../../wasm/regex/windmill_parser_wasm.js"
    );

    inferedSchema = JSON.parse(parse_mysql(content));
    inferedSchema.args = [
      { name: "database", typ: { resource: "mysql" } },
      ...inferedSchema.args,
    ];
  } else if (language === "bigquery") {
    const { parse_bigquery } = await import(
      "../../wasm/regex/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_bigquery(content));
    inferedSchema.args = [
      { name: "database", typ: { resource: "bigquery" } },
      ...inferedSchema.args,
    ];
  } else if (language === "oracledb") {
    const { parse_oracledb } = await import(
      "../../wasm/regex/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_oracledb(content));
    inferedSchema.args = [
      { name: "database", typ: { resource: "oracledb" } },
      ...inferedSchema.args,
    ];
  } else if (language === "snowflake") {
    const { parse_snowflake } = await import(
      "../../wasm/regex/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_snowflake(content));
    inferedSchema.args = [
      { name: "database", typ: { resource: "snowflake" } },
      ...inferedSchema.args,
    ];
  } else if (language === "mssql") {
    const { parse_mssql } = await import(
      "../../wasm/regex/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_mssql(content));
    inferedSchema.args = [
      { name: "database", typ: { resource: "ms_sql_server" } },
      ...inferedSchema.args,
    ];
  } else if (language === "postgresql") {
    const { parse_sql } = await import(
      "../../wasm/regex/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_sql(content));
    inferedSchema.args = [
      { name: "database", typ: { resource: "postgresql" } },
      ...inferedSchema.args,
    ];
  } else if (language === "duckdb") {
    const { parse_duckdb } = await import(
      "../../wasm/regex/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_duckdb(content));
  } else if (language === "graphql") {
    const { parse_graphql } = await import(
      "../../wasm/regex/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_graphql(content));
    inferedSchema.args = [
      { name: "api", typ: { resource: "graphql" } },
      ...inferedSchema.args,
    ];
  } else if (language === "bash") {
    const { parse_bash } = await import(
      "../../wasm/regex/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_bash(content));
  } else if (language === "powershell") {
    const { parse_powershell } = await import(
      "../../wasm/regex/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_powershell(content));
  } else if (language === "php") {
    const { parse_php } = await import(
      "../../wasm/php/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_php(content));
  } else if (language === "rust") {
    const { parse_rust } = await import(
      "../../wasm/rust/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_rust(content));
  } else if (language === "csharp") {
    const { parse_csharp } = await import(
      "../../wasm/csharp/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_csharp(content));
  } else if (language === "nu") {
    const { parse_nu } = await import("../../wasm/nu/windmill_parser_wasm.js");
    inferedSchema = JSON.parse(parse_nu(content));
  } else if (language === "ansible") {
    const { parse_ansible } = await import(
      "../../wasm/yaml/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_ansible(content));
  } else if (language === "java") {
    const { parse_java } = await import(
      "../../wasm/java/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_java(content));
  } else if (language === "ruby") {
    const { parse_ruby } = await import(
      "../../wasm/ruby/windmill_parser_wasm.js"
    );
    inferedSchema = JSON.parse(parse_ruby(content));
    // for related places search: ADD_NEW_LANG
  } else {
    throw new Error("Invalid language: " + language);
  }
  if (inferedSchema.type == "Invalid") {
    log.info(
      colors.yellow(
        `Script ${path} invalid, it cannot be parsed to infer schema.`
      )
    );
    return {
      schema: defaultScriptMetadata().schema,
      has_preprocessor: false,
      no_main_func: false,
    };
  }

  if (!currentSchema) {
    currentSchema = {};
  }
  currentSchema.required = [];
  const oldProperties = JSON.parse(
    JSON.stringify(currentSchema?.properties ?? {})
  );
  currentSchema.properties = {};

  for (const arg of inferedSchema.args) {
    if (!(arg.name in oldProperties)) {
      currentSchema.properties[arg.name] = { description: "", type: "" };
    } else {
      currentSchema.properties[arg.name] = oldProperties[arg.name];
    }
    currentSchema.properties[arg.name] = sortObject(
      currentSchema.properties[arg.name]
    );

    argSigToJsonSchemaType(arg.typ, currentSchema.properties[arg.name]);

    currentSchema.properties[arg.name].default = arg.default;

    if (!arg.has_default && !currentSchema.required.includes(arg.name)) {
      currentSchema.required.push(arg.name);
    }
  }

  return {
    schema: currentSchema,
    has_preprocessor: inferedSchema.has_preprocessor,
    no_main_func: inferedSchema.no_main_func,
  };
}

function sortObject(obj: any): any {
  return Object.keys(obj)
    .sort()
    .reduce(
      (acc, key) => ({
        ...acc,
        [key]: obj[key],
      }),
      {}
    );
}

////////////////////////////////////////////////////////////////////////////////////////////
// end of refactoring TODO                                                                //
////////////////////////////////////////////////////////////////////////////////////////////

export function replaceLock(o?: { lock?: string | string[] }) {
  if (Array.isArray(o?.lock)) {
    o.lock = o.lock.join("\n");
  }
  if (o?.lock?.startsWith("!inline ")) {
    try {
      const lockPath = o?.lock?.split(" ")[1];
      o.lock = readInlinePathSync(lockPath);
    } catch (e) {
      log.info(
        colors.yellow(`Failed to read lockfile, doing as if it was empty: ${e}`)
      );
      o.lock = "";
    }
  }
}
export async function parseMetadataFile(
  scriptPath: string,
  generateMetadataIfMissing:
    | (GlobalOptions & {
        path: string;
        workspaceRemote: Workspace;
        schemaOnly?: boolean;
        rawWorkspaceDependencies: Record<string, string>;
        codebases: SyncCodebase[]
      })
    | undefined
): Promise<{ isJson: boolean; payload: any; path: string }> {
  let metadataFilePath = scriptPath + ".script.json";
  try {
    await Deno.stat(metadataFilePath);
    return {
      path: metadataFilePath,
      payload: JSON.parse(await Deno.readTextFile(metadataFilePath)),
      isJson: true,
    };
  } catch {
    try {
      metadataFilePath = scriptPath + ".script.yaml";
      await Deno.stat(metadataFilePath);
      const payload: any = await yamlParseFile(metadataFilePath);
      replaceLock(payload);

      return {
        path: metadataFilePath,
        payload,
        isJson: false,
      };
    } catch {
      // no metadata file at all. Create it
      log.info(
        (await blueColor())(
          `Creating script metadata file for ${metadataFilePath}`
        )
      );
      metadataFilePath = scriptPath + ".script.yaml";
      let scriptInitialMetadata = defaultScriptMetadata();
      const lockPath = scriptPath + ".script.lock";
      scriptInitialMetadata.lock = "!inline " + lockPath;
      const scriptInitialMetadataYaml = yamlStringify(
        scriptInitialMetadata as Record<string, any>,
        yamlOptions
      );

      await Deno.writeTextFile(metadataFilePath, scriptInitialMetadataYaml, {
        createNew: true,
      });
      await Deno.writeTextFile(lockPath, "", {
        createNew: true,
      });

      if (generateMetadataIfMissing) {
        log.info(
          (await blueColor())(
            `Generating lockfile and schema for ${metadataFilePath}`
          )
        );
        try {
          await generateScriptMetadataInternal(
            generateMetadataIfMissing.path,
            generateMetadataIfMissing.workspaceRemote,
            generateMetadataIfMissing,
            false,
            false,
            generateMetadataIfMissing.rawWorkspaceDependencies,
            generateMetadataIfMissing.codebases,
            false
          );
          scriptInitialMetadata = (await yamlParseFile(
            metadataFilePath
          )) as ScriptMetadata;
          if (!generateMetadataIfMissing.schemaOnly) {
            replaceLock(scriptInitialMetadata);
          }
        } catch (e) {
          log.info(
            colors.yellow(
              `Failed to generate lockfile and schema for ${metadataFilePath}: ${e}`
            )
          );
        }
      }
      return {
        path: metadataFilePath,
        payload: scriptInitialMetadata,
        isJson: false,
      };
    }
  }
}

interface Lock {
  version?: "v2";
  locks?: { [path: string]: string | { [subpath: string]: string } };
}

const WMILL_LOCKFILE = "wmill-lock.yaml";
export async function readLockfile(): Promise<Lock> {
  try {
    const read = await yamlParseFile(WMILL_LOCKFILE);
    if (typeof read == "object" && read != null) {
      return read as Lock;
    } else {
      throw new Error("Invalid lockfile");
    }
  } catch {
    const lock = { locks: {}, version: "v2" as const };
    await Deno.writeTextFile(WMILL_LOCKFILE, yamlStringify(lock, yamlOptions));
    log.info(colors.green("wmill-lock.yaml created"));

    return lock;
  }
}

function v2LockPath(path: string, subpath?: string) {
  if (subpath) {
    return `${path}+${subpath}`;
  } else {
    return path;
  }
}
export async function checkifMetadataUptodate(
  path: string,
  hash: string,
  conf: Lock | undefined,
  subpath?: string
) {
  if (!conf) {
    conf = await readLockfile();
  }
  if (!conf.locks) {
    return false;
  }
  const isV2 = conf?.version == "v2";

  if (isV2) {
    const current = conf.locks?.[v2LockPath(path, subpath)];
    return current == hash;
  } else {
    const obj = conf.locks?.[path];
    const current = subpath && typeof obj == "object" ? obj?.[subpath] : obj;
    return current == hash;
  }
}

export async function generateScriptHash(
  rawWorkspaceDependencies: Record<string, string>,
  scriptContent: string,
  newMetadataContent: string
) {
  return await generateHash(
    JSON.stringify(rawWorkspaceDependencies) + scriptContent + newMetadataContent
  );
}

export async function clearGlobalLock(path: string): Promise<void> {
  const conf = await readLockfile();
  if (!conf?.locks) {
    conf.locks = {};
  }
  const isV2 = conf?.version == "v2";

  if (isV2) {
    // Remove the specific v2 lock entry
    const key = v2LockPath(path);
    if (conf.locks) {
      Object.keys(conf.locks).forEach((k) => {
        if (conf.locks) {
          if (k.startsWith(key)) {
            delete conf.locks[k];
          }
        }
      });
    }
    await Deno.writeTextFile(
      WMILL_LOCKFILE,
      yamlStringify(conf as Record<string, any>, yamlOptions)
    );
  }
}

export async function updateMetadataGlobalLock(
  path: string,
  hash: string,
  subpath?: string
): Promise<void> {
  const conf = await readLockfile();
  if (!conf?.locks) {
    conf.locks = {};
  }
  const isV2 = conf?.version == "v2";

  if (isV2) {
    conf.locks[v2LockPath(path, subpath)] = hash;
  } else {
    if (subpath) {
      let prev: any = conf.locks[path];
      if (!prev || typeof prev != "object") {
        prev = {};
        conf.locks[path] = prev;
      }
      prev[subpath] = hash;
    } else {
      conf.locks[path] = hash;
    }
  }
  await Deno.writeTextFile(
    WMILL_LOCKFILE,
    yamlStringify(conf as Record<string, any>, yamlOptions)
  );
}
