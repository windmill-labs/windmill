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

/**
 * Filters raw workspace dependencies to only include those that:
 * 1. Match the given script language
 * 2. Are referenced by the script's workspace dependency annotation, OR
 *    are the default dependency (no name) when there's no annotation
 */
export function filterWorkspaceDependencies(
  rawWorkspaceDependencies: Record<string, string>,
  scriptContent: string,
  language: ScriptLanguage
): Record<string, string> {
  const wda = extractWorkspaceDepsAnnotation(scriptContent, language);
  const filtered: Record<string, string> = {};

  for (const [depPath, depContent] of Object.entries(rawWorkspaceDependencies)) {
    const depInfo = workspaceDependenciesPathToLanguageAndFilename(depPath);

    if (depInfo && depInfo.language === language) {
      if ((wda && wda.external.includes(depInfo.name ?? "default")) || (wda == null && depInfo.name == undefined)) {
        filtered[depPath] = depContent;
      }
    }
  }

  return filtered;
}

export interface InlineScriptInfo {
  content: string;
  language: ScriptLanguage;
}

/**
 * Filters workspace dependencies for multiple scripts, resolving !inline refs and computing union.
 * Common helper used by flows and apps.
 */
export async function filterWorkspaceDependenciesForScripts(
  scripts: InlineScriptInfo[],
  rawWorkspaceDependencies: Record<string, string>,
  folder: string,
  sep: string
): Promise<Record<string, string>> {
  const filtered: Record<string, string> = {};

  for (const script of scripts) {
    let content = script.content;

    // Resolve !inline reference to actual content
    if (content.startsWith("!inline ")) {
      const filePath = folder + sep + content.replace("!inline ", "");
      try {
        content = await Deno.readTextFile(filePath);
      } catch {
        continue;
      }
    }

    const scriptFiltered = filterWorkspaceDependencies(
      rawWorkspaceDependencies,
      content,
      script.language
    );

    for (const [depPath, depContent] of Object.entries(scriptFiltered)) {
      filtered[depPath] = depContent;
    }
  }

  return filtered;
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


  const metadataWithType = await parseMetadataFile(
    remotePath,
    undefined,
  );

  // read script content
  const scriptContent = await Deno.readTextFile(scriptPath);
  const metadataContent = await Deno.readTextFile(metadataWithType.path);

  const filteredRawWorkspaceDependencies = filterWorkspaceDependencies(
    rawWorkspaceDependencies,
    scriptContent,
    language
  );


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
        filteredRawWorkspaceDependencies
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

// ---------------------------------------------------------------------------
// Annotation parser — mirrors backend's WorkspaceDependenciesAnnotatedRefs::parse
// (windmill-common/src/workspace_dependencies.rs) so the cache key captures
// exactly the parts of scriptContent that affect lockfile generation.
// ---------------------------------------------------------------------------

type AnnotationMode = "manual" | "extra";

interface WorkspaceDepsAnnotation {
  mode: AnnotationMode;
  external: string[];
  inline: string | null;
}

const LANG_ANNOTATION_CONFIG: Partial<
  Record<ScriptLanguage, { comment: string; keyword: string; validityRe?: RegExp }>
> = {
  python3: { comment: "#", keyword: "requirements", validityRe: /^#\s?(\S+)\s*$/ },
  bun: { comment: "//", keyword: "package_json" },
  nativets: { comment: "//", keyword: "package_json" },
  go: { comment: "//", keyword: "go_mod" },
  php: { comment: "//", keyword: "composer_json" },
};

export function extractWorkspaceDepsAnnotation(
  scriptContent: string,
  language: ScriptLanguage,
): WorkspaceDepsAnnotation | null {
  const config = LANG_ANNOTATION_CONFIG[language];
  if (!config) return null;

  const { comment, keyword, validityRe } = config;
  const extraMarker = `extra_${keyword}:`;
  const manualMarker = `${keyword}:`;

  const lines = scriptContent.split("\n");

  // Find first annotation line (mirrors Rust find_position)
  let pos = -1;
  for (let i = 0; i < lines.length; i++) {
    const l = lines[i];
    if (l.startsWith(comment) && (l.includes(extraMarker) || l.includes(manualMarker))) {
      pos = i;
      break;
    }
  }
  if (pos === -1) return null;

  const annotationLine = lines[pos];
  const mode: AnnotationMode = annotationLine.includes(extraMarker) ? "extra" : "manual";

  // Parse external references from the annotation line
  const marker = mode === "extra" ? extraMarker : manualMarker;
  const unparsed = annotationLine.replaceAll(marker, "").replaceAll(comment, "");
  const external = unparsed
    .split(",")
    .map((s) => s.trim())
    .filter((s) => s.length > 0);

  // Parse inline deps from subsequent lines
  const inlineParts: string[] = [];
  for (let i = pos + 1; i < lines.length; i++) {
    const l = lines[i];
    if (validityRe) {
      const match = validityRe.exec(l);
      if (match && match[1]) {
        inlineParts.push(match[1]);
      } else {
        break;
      }
    } else {
      if (!l.startsWith(comment)) {
        break;
      }
      inlineParts.push(l.substring(comment.length));
    }
  }

  const inlineStr = inlineParts.join("\n");
  const inline = inlineStr.trim().length > 0 ? inlineStr : null;

  return { mode, external, inline };
}

export async function computeLockCacheKey(
  scriptContent: string,
  language: ScriptLanguage,
  rawWorkspaceDependencies: Record<string, string>,
): Promise<string> {
  const annotation = extractWorkspaceDepsAnnotation(scriptContent, language);
  const annotationStr = annotation
    ? `${annotation.mode}|${annotation.external.join(",")}|${annotation.inline ?? ""}`
    : "none";
  const sortedDepsKeys = Object.keys(rawWorkspaceDependencies).sort();
  const depsStr = sortedDepsKeys.map((k) => `${k}=${rawWorkspaceDependencies[k]}`).join(";");
  return await generateHash(`${language}|${annotationStr}|${depsStr}`);
}

const lockCache = new Map<string, string>();

export function clearLockCache(): void {
  lockCache.clear();
}

async function fetchScriptLock(
  workspace: Workspace,
  scriptContent: string,
  language: ScriptLanguage,
  remotePath: string,
  rawWorkspaceDependencies: Record<string, string>,
): Promise<string> {
  const hasRawDeps = Object.keys(rawWorkspaceDependencies).length > 0;
  const cacheKey = hasRawDeps
    ? await computeLockCacheKey(scriptContent, language, rawWorkspaceDependencies)
    : undefined;
  if (cacheKey && lockCache.has(cacheKey)) {
    log.info(`Using cached lockfile for ${remotePath}`);
    return lockCache.get(cacheKey)!;
  }

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
        raw_scripts: [
          {
            raw_code: scriptContent,
            language: language,
            script_path: remotePath,
          },
        ],
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
    if (cacheKey) {
      lockCache.set(cacheKey, lock);
    }
    return lock;
  } catch (e) {
    if (e instanceof LockfileGenerationError) {
      throw e;
    }
    throw new LockfileGenerationError(
      `Failed to generate lockfile:${rawResponse.statusText}, ${responseText}, ${e}`
    );
  }
}

async function updateScriptLock(
  workspace: Workspace,
  scriptContent: string,
  language: ScriptLanguage,
  remotePath: string,
  metadataContent: Record<string, any>,
  rawWorkspaceDependencies: Record<string, string>
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


  if (Object.keys(rawWorkspaceDependencies).length > 0) {
    const dependencyPaths = Object.keys(rawWorkspaceDependencies).join(', ');
    log.info(`Generating script lock for ${remotePath} with raw workspace dependencies: ${dependencyPaths}`);
  }

  const lock = await fetchScriptLock(
    workspace,
    scriptContent,
    language,
    remotePath,
    rawWorkspaceDependencies,
  );

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

/**
 * Normalizes a path to use Linux separators (forward slashes).
 * This ensures wmill-lock.yaml is portable across Windows and Linux.
 */
export function normalizeLockPath(p: string): string {
  return p.replace(/\\/g, "/");
}

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
  const normalizedPath = normalizeLockPath(path);
  if (subpath) {
    return `${normalizedPath}+${normalizeLockPath(subpath)}`;
  } else {
    return normalizedPath;
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
