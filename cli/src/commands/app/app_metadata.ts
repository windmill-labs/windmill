import path from "node:path";
import { readFile, mkdir } from "node:fs/promises";
import { colors } from "@cliffy/ansi/colors";
import * as log from "@std/log";
import { SEPARATOR as SEP } from "@std/path";
import { yamlParseFile } from "../../utils/yaml.ts";
import { stringify as yamlStringify } from "@std/yaml";
import { GlobalOptions } from "../../types.ts";
import {
  checkifMetadataUptodate,
  blueColor,
  clearGlobalLock,
  updateMetadataGlobalLock,
  inferSchema,
  getRawWorkspaceDependencies,
  normalizeLockPath,
  filterWorkspaceDependencies,
  filterWorkspaceDependenciesForScripts,
  InlineScriptInfo,
} from "../../utils/metadata.ts";
import {
  ScriptLanguage,
  workspaceDependenciesLanguages,
} from "../../utils/script_common.ts";
import { generateHash, getHeaders, writeIfChanged } from "../../utils/utils.ts";
import { exts } from "../script/script.ts";
import { FSFSElement, yamlOptions } from "../sync/sync.ts";
import { Workspace } from "../workspace/workspace.ts";
import {
  AppFile as RawAppFile,
  loadRunnablesFromBackend,
  writeRunnableToBackend,
} from "./raw_apps.ts";
import { replaceInlineScripts, AppFile as NormalAppFile } from "./app.ts";
import {
  newPathAssigner,
  newRawAppPathAssigner,
  SupportedLanguage,
} from "../../../windmill-utils-internal/src/path-utils/path-assigner.ts";
import { mergeConfigWithConfigFile, SyncOptions } from "../../core/conf.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { requireLogin } from "../../core/auth.ts";
import { getNonDottedPaths } from "../../utils/resource_folders.ts";

const TOP_HASH = "__app_hash";
export const APP_BACKEND_FOLDER = "backend";

// Union type for app files that can be either raw or normal apps
type AppFile = RawAppFile | NormalAppFile;

/**
 * Generates a hash for all inline scripts in an app directory
 */
async function generateAppHash(
  rawReqs: Record<string, string> | undefined,
  folder: string,
  rawApp: boolean,
  defaultTs: "bun" | "deno" | undefined
): Promise<Record<string, string>> {
  const runnablesFolder = rawApp
    ? path.join(folder, APP_BACKEND_FOLDER)
    : folder;
  const hashes: Record<string, string> = {};

  try {
    const elems = await FSFSElement(runnablesFolder, [], true);
    for await (const f of elems.getChildren()) {
      // For normal apps, skip non-script files (metadata files like app.yaml)
      // For raw apps, all files in backend/ are scripts
      if (!rawApp) {
        const isMetadataFile = f.path.endsWith("app.yaml") || f.path.endsWith("app.json");
        if (isMetadataFile) {
          continue;
        }
      }
      if (exts.some((e) => f.path.endsWith(e))) {
        // Embed lock into hash
        // Normalize path to ensure OS-independent hashing
        const relativePath = normalizeLockPath(f.path.replace(runnablesFolder + SEP, ""));
        hashes[relativePath] = await generateHash(
          (await f.getContentText()) + JSON.stringify(rawReqs)
        );
      }
    }
  } catch (error: any) {
    // If runnables folder doesn't exist, that's okay
    if (error.code !== "ENOENT") {
      throw error;
    }
  }

  return { ...hashes, [TOP_HASH]: await generateHash(JSON.stringify(hashes)) };
}

/**
 * Updates locks for inline scripts in an app
 */
export async function generateAppLocksInternal(
  appFolder: string,
  rawApp: boolean,
  dryRun: boolean,
  workspace: Workspace,
  opts: GlobalOptions & {
    defaultTs?: "bun" | "deno";
  },
  justUpdateMetadataLock?: boolean,
  noStaleMessage?: boolean
): Promise<string | void> {
  if (appFolder.endsWith(SEP)) {
    appFolder = appFolder.substring(0, appFolder.length - 1);
  }

  const remote_path = appFolder.replaceAll(SEP, "/");

  if (!justUpdateMetadataLock && !noStaleMessage) {
    log.info(`Generating locks for app ${appFolder} at ${remote_path}`);
  }

  const rawWorkspaceDependencies: Record<string, string> =
    await getRawWorkspaceDependencies();

  // Read the app file first to filter workspace dependencies
  const appFilePath = path.join(
    appFolder,
    rawApp ? "raw_app.yaml" : "app.yaml"
  );
  const appFile = (await yamlParseFile(appFilePath)) as AppFile;

  // Filter workspace dependencies based on inline scripts' languages and annotations
  const appValue = rawApp ? (appFile as RawAppFile).runnables : (appFile as NormalAppFile).value;
  const filteredDeps = await filterWorkspaceDependenciesForApp(
    appValue,
    rawWorkspaceDependencies,
    appFolder
  );

  let hashes = await generateAppHash(
    filteredDeps,
    appFolder,
    rawApp,
    opts.defaultTs
  );

  const conf = await import("../../utils/metadata.ts").then((m) =>
    m.readLockfile()
  );
  if (
    await checkifMetadataUptodate(appFolder, hashes[TOP_HASH], conf, TOP_HASH)
  ) {
    if (!noStaleMessage) {
      log.info(
        colors.green(`App ${remote_path} metadata is up-to-date, skipping`)
      );
    }
    return;
  } else if (dryRun) {
    return remote_path;
  }

  if (Object.keys(filteredDeps).length > 0) {
    log.info(
      (await blueColor())(
        `Found workspace dependencies (${workspaceDependenciesLanguages
          .map((l) => l.filename)
          .join("/")}) for ${appFolder}, using them`
      )
    );
  }

  if (!justUpdateMetadataLock) {
    const changedScripts = [];
    // Find hashes that do not correspond to previous hashes
    for (const [scriptPath, hash] of Object.entries(hashes)) {
      if (scriptPath == TOP_HASH) {
        continue;
      }
      if (!(await checkifMetadataUptodate(appFolder, hash, conf, scriptPath))) {
        changedScripts.push(scriptPath);
      }
    }

    if (changedScripts.length > 0) {
      log.info(
        `Recomputing locks of ${changedScripts.join(", ")} in ${appFolder}`
      );

      if (rawApp) {
        const runnablesPath = path.join(appFolder, APP_BACKEND_FOLDER);

        // Load runnables from separate files (new format) or fall back to raw_app.yaml (old format)
        const rawAppFile = appFile as RawAppFile;
        let runnables = await loadRunnablesFromBackend(runnablesPath);
        if (Object.keys(runnables).length === 0 && rawAppFile.runnables) {
          // Fall back to old format
          runnables = rawAppFile.runnables;
        }

        // Replace inline scripts for changed runnables
        replaceInlineScripts(runnables, runnablesPath + SEP, false);

        // Update the app runnables with new locks (writes to separate files)
        await updateRawAppRunnables(
          workspace,
          runnables,
          remote_path,
          appFolder,
          filteredDeps,
          opts.defaultTs
        );
        // Note: updateRawAppRunnables now writes each runnable to its own file
      } else {
        const normalAppFile = appFile as NormalAppFile;

        // Replace inline scripts for normal apps
        replaceInlineScripts(normalAppFile.value, appFolder + SEP, false);

        // Update the app value with new locks
        normalAppFile.value = await updateAppInlineScripts(
          workspace,
          normalAppFile.value,
          remote_path,
          appFolder,
          filteredDeps,
          opts.defaultTs
        );

        // Write the updated app file (only for normal apps, raw apps use separate files)
        writeIfChanged(
          appFilePath,
          yamlStringify(appFile as Record<string, any>, yamlOptions)
        );
      }
    } else {
      log.info(colors.gray(`No scripts changed in ${appFolder}`));
    }
  }

  // Regenerate hashes after updates
  hashes = await generateAppHash(
    filteredDeps,
    appFolder,
    rawApp,
    opts.defaultTs
  );
  await clearGlobalLock(appFolder);
  for (const [scriptPath, hash] of Object.entries(hashes)) {
    await updateMetadataGlobalLock(appFolder, hash, scriptPath);
  }
  log.info(colors.green(`App ${remote_path} lockfiles updated`));
}

/**
 * Callback type for processing inline scripts found during traversal
 */
type InlineScriptProcessor = (
  inlineScript: any,
  context: {
    path: string[];
    parentKey: string;
    parentObject: any;
  }
) => Promise<any>;

/**
 * Filters raw workspace dependencies for an app by traversing all inline scripts,
 * filtering deps for each based on language and annotations, then computing the union.
 */
export async function filterWorkspaceDependenciesForApp(
  appValue: any,
  rawWorkspaceDependencies: Record<string, string>,
  folder: string
): Promise<Record<string, string>> {
  // Collect all inline scripts (use clone to avoid any mutations)
  const scripts: InlineScriptInfo[] = [];

  await traverseAndProcessInlineScripts(structuredClone(appValue), async (inlineScript) => {
    if (inlineScript.content && inlineScript.language) {
      scripts.push({
        content: inlineScript.content,
        language: inlineScript.language as ScriptLanguage,
      });
    }
    return inlineScript;
  });

  return await filterWorkspaceDependenciesForScripts(scripts, rawWorkspaceDependencies, folder, SEP);
}

/**
 * Traverses an app structure (either app.value for normal apps or app.runnables for raw apps)
 * and processes all inline scripts found, returning the updated structure
 */
async function traverseAndProcessInlineScripts(
  obj: any,
  processor: InlineScriptProcessor,
  currentPath: string[] = []
): Promise<any> {
  if (!obj || typeof obj !== "object") {
    return obj;
  }

  if (Array.isArray(obj)) {
    return await Promise.all(
      obj.map((item, index) =>
        traverseAndProcessInlineScripts(item, processor, [
          ...currentPath,
          `[${index}]`,
        ])
      )
    );
  }

  const result: Record<string, any> = {};

  for (const [key, value] of Object.entries(obj)) {
    if (key === "inlineScript" && typeof value === "object") {
      // Found an inline script - process it
      result[key] = await processor(value, {
        path: currentPath,
        parentKey: key,
        parentObject: obj,
      });
    } else {
      // Recursively process nested objects
      result[key] = await traverseAndProcessInlineScripts(value, processor, [
        ...currentPath,
        key,
      ]);
    }
  }

  return result;
}

/**
 * Updates locks for all runnables in a raw app, generating locks inline script by inline script.
 * Writes each runnable to its own YAML file in the backend folder (new format).
 * Also writes content and lock files to the runnables folder.
 */
async function updateRawAppRunnables(
  workspace: Workspace,
  runnables: Record<string, any>,
  remotePath: string,
  appFolder: string,
  rawDeps?: Record<string, string>,
  defaultTs: "bun" | "deno" = "bun"
): Promise<void> {
  const runnablesFolder = path.join(appFolder, APP_BACKEND_FOLDER);

  // Ensure runnables folder exists
  try {
    await mkdir(runnablesFolder, { recursive: true });
  } catch {
    // Folder may already exist
  }

  const pathAssigner = newRawAppPathAssigner(defaultTs);

  for (const [runnableId, runnable] of Object.entries(runnables)) {
    // Only process inline scripts (runnableByName/inline with inlineScript)
    if (
      (runnable?.type !== "runnableByName" && runnable?.type !== "inline") ||
      !runnable?.inlineScript
    ) {
      // Write non-inline runnables to their own file as-is
      writeRunnableToBackend(runnablesFolder, runnableId, runnable);
      continue;
    }

    const inlineScript = runnable.inlineScript;
    const language = inlineScript.language as SupportedLanguage;
    const content = inlineScript.content;

    if (!content || !language) {
      writeRunnableToBackend(runnablesFolder, runnableId, runnable);
      continue;
    }

    // Skip if content is still an !inline reference (should have been replaced by replaceInlineScripts)
    if (typeof content === "string" && content.startsWith("!inline ")) {
      log.warn(
        colors.yellow(
          `Runnable ${runnableId} content is still an !inline reference, skipping`
        )
      );
      writeRunnableToBackend(runnablesFolder, runnableId, runnable);
      continue;
    }

    // Skip frontend scripts - they don't need locks
    if (language === "frontend") {
      // Still need to write the runnable YAML file
      const [basePathO, ext] = pathAssigner.assignPath(
        runnable.name ?? runnableId,
        language
      );
      const basePath = basePathO.replaceAll(SEP, "/");
      const contentPath = path.join(runnablesFolder, `${basePath}${ext}`);
      writeIfChanged(contentPath, content);

      // Write simplified runnable YAML - just type: 'inline' plus metadata
      // inlineScript is not needed since content/language can be derived from files
      const simplifiedRunnable: Record<string, any> = { type: "inline" };
      for (const [key, value] of Object.entries(runnable)) {
        if (key !== "inlineScript" && key !== "type") {
          simplifiedRunnable[key] = value;
        }
      }
      writeRunnableToBackend(runnablesFolder, runnableId, simplifiedRunnable);
      continue;
    }

    log.info(
      colors.gray(
        `Generating lock for runnable ${runnableId} (${language})
        }`
      )
    );

    try {
      const lock = await generateInlineScriptLock(
        workspace,
        content,
        language,
        `${remotePath}/${runnableId}`,
        rawDeps
      );

      // Determine file extension for this language
      const [basePathO, ext] = pathAssigner.assignPath(
        runnable.name ?? runnableId,
        language
      );
      const basePath = basePathO.replaceAll(SEP, "/");
      const contentPath = path.join(runnablesFolder, `${basePath}${ext}`);
      const lockPath = path.join(runnablesFolder, `${basePath}lock`);

      // Write content to file
      writeIfChanged(contentPath, content);

      // Write lock to file if it exists
      if (lock && lock !== "") {
        writeIfChanged(lockPath, lock);
      }

      // Write simplified runnable YAML - just type: 'inline' plus metadata
      // inlineScript is not needed since content/lock/language can be derived from files
      const simplifiedRunnable: Record<string, any> = { type: "inline" };
      for (const [key, value] of Object.entries(runnable)) {
        if (key !== "inlineScript" && key !== "type") {
          simplifiedRunnable[key] = value;
        }
      }

      // Write the runnable to its own YAML file
      writeRunnableToBackend(runnablesFolder, runnableId, simplifiedRunnable);

      log.info(
        colors.gray(
          `  Written ${runnableId}.yaml, ${basePath}${ext}${lock ? ` and ${basePath}lock` : ""}`
        )
      );
    } catch (error: any) {
      log.error(
        colors.red(
          `Failed to generate lock for runnable ${runnableId}: ${error.message}`
        )
      );
      // Write the original runnable even if lock generation fails
      writeRunnableToBackend(runnablesFolder, runnableId, runnable);
    }
  }
}

/**
 * Updates locks for all inline scripts in a normal app, similar to updateRawAppRunnables
 * but for the app.value structure instead of app.runnables
 */
async function updateAppInlineScripts(
  workspace: Workspace,
  appValue: any,
  remotePath: string,
  appFolder: string,
  rawDeps?: Record<string, string>,
  defaultTs: "bun" | "deno" = "bun"
): Promise<any> {
  const pathAssigner = newPathAssigner(defaultTs, { skipInlineScriptSuffix: getNonDottedPaths() });

  const processor: InlineScriptProcessor = async (inlineScript, context) => {
    const language = inlineScript.language as SupportedLanguage;
    const content = inlineScript.content;

    if (!content || !language) {
      return inlineScript;
    }

    // Skip if content is still an !inline reference (should have been replaced by replaceInlineScripts)
    if (typeof content === "string" && content.startsWith("!inline ")) {
      log.warn(
        colors.yellow(
          `Inline script at ${context.path.join(
            "."
          )} is still an !inline reference, skipping`
        )
      );
      return inlineScript;
    }

    // Get the name from the parent object (following extractInlineScriptsForApps pattern)
    // For normal apps, the name is stored in the component's "name" property
    const scriptName = context.parentObject?.["name"] || "unnamed";
    const scriptPath = `${remotePath}/${context.path.join("/")}`;

    try {
      let lock: string | undefined;
      if (language !== "frontend") {
        log.info(
          colors.gray(
            `Generating lock for inline script "${scriptName}" at ${context.path.join(
              "."
            )} (${language})`
          )
        );

        lock = await generateInlineScriptLock(
          workspace,
          content,
          language,
          scriptPath,
          rawDeps
        );
      }
      // Determine file extension for this language (following extractInlineScriptsForApps pattern)
      const [basePathO, ext] = pathAssigner.assignPath(scriptName, language);
      const basePath = basePathO.replaceAll(SEP, "/");
      const contentPath = path.join(appFolder, `${basePath}${ext}`);
      const lockPath = path.join(appFolder, `${basePath}lock`);

      // Write content to file
      writeIfChanged(contentPath, content);

      // Write lock to file if it exists
      if (lock && lock !== "") {
        writeIfChanged(lockPath, lock);
      }

      // Update the inline script with !inline references
      const inlineContentRef = `!inline ${basePath}${ext}`;
      const inlineLockRef =
        lock && lock !== "" ? `!inline ${basePath}lock` : "";

      log.info(
        colors.gray(
          `  Written ${basePath}${ext}${lock ? ` and ${basePath}lock` : ""}`
        )
      );

      return {
        ...inlineScript,
        content: inlineContentRef,
        ...(lock ? { lock: inlineLockRef } : {}),
      };
    } catch (error: any) {
      log.error(
        colors.red(
          `Failed to generate lock for inline script at ${context.path.join(
            "."
          )}: ${error.message}`
        )
      );
      // Return original on error
      return inlineScript;
    }
  };

  return await traverseAndProcessInlineScripts(appValue, processor);
}

/**
 * Generates a lock for a single inline script using the dependencies endpoint
 */
async function generateInlineScriptLock(
  workspace: Workspace,
  content: string,
  language: string,
  scriptPath: string,
  rawWorkspaceDependencies: Record<string, string> | undefined
): Promise<string> {
  // Filter workspace dependencies to only include those matching this script's language and annotations
  const filteredDeps = rawWorkspaceDependencies
    ? filterWorkspaceDependencies(rawWorkspaceDependencies, content, language as ScriptLanguage)
    : undefined;

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
            raw_code: content,
            language: language,
            script_path: scriptPath,
          },
        ],
        raw_workspace_dependencies:
          filteredDeps && Object.keys(filteredDeps).length > 0
            ? filteredDeps
            : null,
        entrypoint: scriptPath,
      }),
    }
  );

  if (!rawResponse.ok) {
    const text = await rawResponse.text();
    throw new Error(
      `Dependency generation failed: ${rawResponse.status} ${rawResponse.statusText}\n${text}`
    );
  }

  let responseText = "reading response failed";
  try {
    responseText = await rawResponse.text();
    const response = JSON.parse(responseText);
    const lock = response.lock;

    if (lock === undefined) {
      if (response?.["error"]?.["message"]) {
        throw new Error(
          `Failed to generate lockfile: ${response?.["error"]?.["message"]}`
        );
      }
      throw new Error(
        `Failed to generate lockfile: ${JSON.stringify(response, null, 2)}`
      );
    }

    return lock ?? "";
  } catch (e: any) {
    throw new Error(
      `Failed to parse dependency response: ${rawResponse.statusText}, ${responseText}, ${e.message}`
    );
  }
}

/**
 * Result of schema inference for a runnable
 */
export interface InferredSchemaResult {
  runnableId: string;
  schema: any;
}

/**
 * Infers schema for a single runnable from its file content.
 * Used by dev server to update schema in memory (for wmill.d.ts generation).
 * Does NOT write to the runnable YAML file - schema is kept in memory only.
 *
 * @param appFolder - The folder containing the raw app
 * @param runnableFilePath - The path to the changed runnable file (relative to runnables folder)
 * @returns The runnable ID and inferred schema, or undefined if inference failed/not applicable
 */
export async function inferRunnableSchemaFromFile(
  appFolder: string,
  runnableFilePath: string
): Promise<InferredSchemaResult | undefined> {
  // Extract runnable ID from file path (e.g., "myRunnable.ts" -> "myRunnable")
  const fileName = path.basename(runnableFilePath);

  // Skip lock files and yaml files (runnable metadata)
  if (fileName.endsWith(".lock") || fileName.endsWith(".yaml")) {
    return undefined;
  }

  // Match pattern: {runnableId}.{ext} - extract the runnable ID (everything before the last dot)
  const match = fileName.match(/^(.+)\.[^.]+$/);
  if (!match) {
    return undefined;
  }

  const runnableId = match[1];

  // Read the runnable from its separate YAML file (new format)
  const runnableFilePath2 = path.join(
    appFolder,
    APP_BACKEND_FOLDER,
    `${runnableId}.yaml`
  );

  let runnable: any;
  try {
    runnable = await yamlParseFile(runnableFilePath2);
  } catch {
    // Fall back to reading from raw_app.yaml (old format)
    try {
      const appFilePath = path.join(appFolder, "raw_app.yaml");
      const appFile = (await yamlParseFile(appFilePath)) as RawAppFile;
      if (!appFile.runnables?.[runnableId]) {
        log.warn(
          colors.yellow(`Runnable ${runnableId} not found in backend folder or raw_app.yaml`)
        );
        return undefined;
      }
      runnable = appFile.runnables[runnableId];
    } catch {
      log.warn(
        colors.yellow(`Could not read runnable ${runnableId} from any source`)
      );
      return undefined;
    }
  }

  // Only process inline scripts
  if (!runnable?.inlineScript) {
    return undefined;
  }

  const inlineScript = runnable.inlineScript;
  const language = inlineScript.language as SupportedLanguage;

  // Read the actual content from the file
  const fullFilePath = path.join(
    appFolder,
    APP_BACKEND_FOLDER,
    runnableFilePath
  );
  let content: string;
  try {
    content = await readFile(fullFilePath, "utf-8");
  } catch {
    log.warn(colors.yellow(`Could not read file: ${fullFilePath}`));
    return undefined;
  }

  // Infer schema from script content
  const currentSchema = inlineScript.schema;
  const remotePath = appFolder.replaceAll(SEP, "/");

  try {
    const schemaResult = await inferSchema(
      language as ScriptLanguage,
      content,
      currentSchema,
      `${remotePath}/${runnableId}`
    );

    log.info(colors.green(`  Inferred schema for ${runnableId}`));
    return {
      runnableId: runnableId,
      schema: schemaResult.schema,
    };
  } catch (schemaError: any) {
    log.warn(
      colors.yellow(
        `Failed to infer schema for ${runnableId}: ${schemaError.message}`
      )
    );
    return undefined;
  }
}

function getAppFolders(elems: Record<string, any>, extension: string) {
  return Object.keys(elems)
    .filter((p) => p.endsWith(SEP + extension))
    .map((p) => p.substring(0, p.length - (SEP + extension).length));
}

export async function generateLocksCommand(
  opts: GlobalOptions & {
    yes?: boolean;
    dryRun?: boolean;
    defaultTs?: "bun" | "deno";
  } & SyncOptions,
  appPath: string | undefined
) {
  const { generateAppLocksInternal } = await import("./app_metadata.ts");
  const { elementsToMap, FSFSElement } = await import("../sync/sync.ts");
  const { ignoreF } = await import("../sync/sync.ts");
  const { Confirm } = await import("@cliffy/prompt/confirm");

  if (appPath == "") {
    appPath = undefined;
  }

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  opts = await mergeConfigWithConfigFile(opts);

  if (appPath) {
    //TODO: Generate metadata for a specific raw app but handle normal apps to
    throw new Error("Not implemented");
    // Generate metadata for a specific app
    // await generateAppLocksInternal(
    //   appPath,
    //   true,
    //   false,
    //   workspace,
    //   opts,
    //   false,
    //   false
    // );
  } else {
    // Generate metadata for all apps
    const ignore = await ignoreF(opts);
    const elems = await elementsToMap(
      await FSFSElement(process.cwd(), [], true),
      (p, isD) => {
        return (
          ignore(p, isD) ||
          (!isD &&
            !p.endsWith(SEP + "raw_app.yaml") &&
            !p.endsWith(SEP + "app.yaml"))
        );
      },
      false,
      {}
    );

    const rawAppFolders = getAppFolders(elems, "raw_app.yaml");
    const appFolders = getAppFolders(elems, "app.yaml");

    let hasAny = false;
    log.info(
      `Checking metadata for all apps (${appFolders.length}) and raw apps (${rawAppFolders.length})`
    );
    for (const appFolder of rawAppFolders) {
      const candidate = await generateAppLocksInternal(
        appFolder,
        true,
        true,
        workspace,
        opts,
        false,
        true
      );
      if (candidate) {
        hasAny = true;
        log.info(colors.green(`+ ${candidate}`));
      }
    }

    for (const appFolder of appFolders) {
      const candidate = await generateAppLocksInternal(
        appFolder,
        false,
        true,
        workspace,
        opts,
        false,
        true
      );
      if (candidate) {
        hasAny = true;
        log.info(colors.green(`+ ${candidate}`));
      }
    }

    if (hasAny) {
      if (opts.dryRun) {
        log.info(colors.gray(`Dry run complete.`));
        return;
      }
      if (
        !opts.yes &&
        !(await Confirm.prompt({
          message: "Update the metadata of the above apps?",
          default: true,
        }))
      ) {
        return;
      }
    } else {
      log.info(colors.green.bold("No metadata to update"));
      return;
    }

    for (const appFolder of rawAppFolders) {
      await generateAppLocksInternal(
        appFolder,
        true,
        false,
        workspace,
        opts,
        false,
        true
      );
    }

    for (const appFolder of appFolders) {
      await generateAppLocksInternal(
        appFolder,
        false,
        false,
        workspace,
        opts,
        false,
        true
      );
    }
  }
}
