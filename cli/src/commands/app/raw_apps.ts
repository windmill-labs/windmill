import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace, validatePath } from "../../core/context.ts";
import { colors } from "@cliffy/ansi/colors";
import * as log from "@std/log";
import { SEPARATOR as SEP } from "@std/path";
import * as windmillUtils from "@windmill-labs/shared-utils";
import { yamlParseFile } from "../../utils/yaml.ts";
import { stringify as yamlStringify } from "@std/yaml";
import * as wmill from "../../../gen/services.gen.ts";
import { Policy } from "../../../gen/types.gen.ts";
import path from "node:path";
import { readFile, readdir } from "node:fs/promises";

import { GlobalOptions, isSuperset } from "../../types.ts";
import { deepEqual } from "../../utils/utils.ts";

import { replaceInlineScripts, repopulateFields } from "./app.ts";
import { createBundle, detectFrameworks } from "./bundle.ts";
import { APP_BACKEND_FOLDER } from "./app_metadata.ts";
import { writeIfChanged } from "../../utils/utils.ts";
import { yamlOptions } from "../sync/sync.ts";
import {
  EXTENSION_TO_LANGUAGE,
  getLanguageFromExtension,
} from "../../../windmill-utils-internal/src/path-utils/path-assigner.ts";

export interface AppFile {
  runnables?: any;
  custom_path?: string;
  public?: boolean;
  summary: string;
  policy: Policy;
  data?: {
    tables?: string[];
    datatable?: string;
    schema?: string;
  };
}

/**
 * Finds the content file for a runnable by looking for files matching the runnableId.
 * Returns the file extension and content, or undefined if not found.
 */
async function findRunnableContentFile(
  backendPath: string,
  runnableId: string,
  allFiles: string[],
): Promise<{ ext: string; content: string } | undefined> {
  // Look for files matching pattern: {runnableId}.{ext}
  // where ext is a known language extension
  for (const fileName of allFiles) {
    // Skip yaml and lock files
    if (fileName.endsWith(".yaml") || fileName.endsWith(".lock")) {
      continue;
    }

    // Check if file starts with runnableId followed by a dot
    if (!fileName.startsWith(runnableId + ".")) {
      continue;
    }

    // Extract extension (everything after the first dot following runnableId)
    const ext = fileName.substring(runnableId.length + 1);

    // Check if this is a recognized extension
    if (EXTENSION_TO_LANGUAGE[ext]) {
      try {
        const content = await readFile(
          path.join(backendPath, fileName), "utf-8",
        );
        return { ext, content };
      } catch {
        continue;
      }
    }
  }
  return undefined;
}

/**
 * Extracts the runnable ID from a code file name.
 * Returns undefined if the file is not a recognized code file.
 *
 * Examples:
 * - "get_user.ts" -> "get_user"
 * - "fetch_data.bun.ts" -> "fetch_data"
 * - "query.pg.sql" -> "query"
 */
function getRunnableIdFromCodeFile(fileName: string): string | undefined {
  // Skip yaml and lock files
  if (fileName.endsWith(".yaml") || fileName.endsWith(".lock")) {
    return undefined;
  }

  // Try to find a matching extension
  for (const ext of Object.keys(EXTENSION_TO_LANGUAGE)) {
    if (fileName.endsWith("." + ext)) {
      return fileName.slice(0, -(ext.length + 1));
    }
  }

  return undefined;
}

/**
 * Loads all runnables from the backend folder.
 *
 * Supports two modes:
 * 1. Explicit YAML config: `<id>.yaml` with type specification + optional `<id>.<ext>` code file
 * 2. Code-only (auto-detect): Just `<id>.<ext>` - assumes type: inline with empty fields
 *
 * Converts from file format to API format:
 * - For inline scripts (type: 'inline'): derives inlineScript from sibling files
 * - For path-based runnables (type: 'script'|'hubscript'|'flow'): converts to API format
 *   e.g., { type: "script" } -> { type: "path", runType: "script" }
 *
 * Returns an empty object if the backend folder doesn't exist.
 *
 * @param backendPath - Path to the backend folder
 * @param defaultTs - Default TypeScript runtime ("bun" or "deno")
 */
export async function loadRunnablesFromBackend(
  backendPath: string,
  defaultTs: "bun" | "deno" = "bun",
): Promise<Record<string, any>> {
  const runnables: Record<string, any> = {};

  try {
    // First, collect all files in the backend folder
    const allFiles: string[] = [];
    const _entries = await readdir(backendPath, { withFileTypes: true });
    for (const entry of _entries) {
      if (entry.isFile()) {
        allFiles.push(entry.name);
      }
    }

    // Track which runnable IDs have been processed (from YAML files)
    const processedIds = new Set<string>();

    // Process YAML files first (explicit configuration)
    for (const fileName of allFiles) {
      if (!fileName.endsWith(".yaml")) {
        continue;
      }

      const runnableId = fileName.replace(".yaml", "");
      processedIds.add(runnableId);

      const filePath = path.join(backendPath, fileName);
      const runnable = (await yamlParseFile(filePath)) as Record<string, any>;

      // If this is an inline script (type: 'inline'), derive inlineScript from files
      if (runnable?.type === "inline") {
        const contentFile = await findRunnableContentFile(
          backendPath,
          runnableId,
          allFiles,
        );

        if (contentFile) {
          const language = getLanguageFromExtension(contentFile.ext, defaultTs);

          // Try to load lock file
          let lock: string | undefined;
          try {
            lock = await readFile(
              path.join(backendPath, `${runnableId}.lock`),
              "utf-8",
            );
          } catch {
            // No lock file, that's fine
          }

          // Reconstruct inlineScript object
          runnable.inlineScript = {
            content: contentFile.content,
            language,
            ...(lock ? { lock } : {}),
          };
        }
      } else if (
        runnable?.type === "script" ||
        runnable?.type === "hubscript" ||
        runnable?.type === "flow"
      ) {
        // For path-based runnables, convert from file format to API format
        // { type: "script" } -> { type: "path", runType: "script" }
        // { type: "hubscript" } -> { type: "path", runType: "hubscript" }
        // { type: "flow" } -> { type: "path", runType: "flow" }
        const { type, schema: _schema, ...rest } = runnable;
        runnable.type = "path";
        runnable.runType = type;
        // Remove schema if present
        delete runnable.schema;
        Object.assign(runnable, rest);
      }

      runnables[runnableId] = runnable;
    }

    // Auto-detect code files without YAML config (assume type: inline)
    for (const fileName of allFiles) {
      const runnableId = getRunnableIdFromCodeFile(fileName);

      if (!runnableId) {
        continue; // Not a recognized code file
      }

      if (processedIds.has(runnableId)) {
        continue; // Already processed via YAML file
      }

      // Found a code file without corresponding YAML - treat as inline runnable
      processedIds.add(runnableId);

      const contentFile = await findRunnableContentFile(
        backendPath,
        runnableId,
        allFiles,
      );

      if (contentFile) {
        const language = getLanguageFromExtension(contentFile.ext, defaultTs);

        // Try to load lock file
        let lock: string | undefined;
        try {
          lock = await readFile(
            path.join(backendPath, `${runnableId}.lock`), "utf-8",
          );
        } catch {
          // No lock file, that's fine
        }

        // Create inline runnable with default empty fields
        runnables[runnableId] = {
          type: "inline",
          inlineScript: {
            content: contentFile.content,
            language,
            ...(lock ? { lock } : {}),
          },
        };
      }
    }
  } catch (error: any) {
    if (error.code !== "ENOENT") {
      throw error;
    }
  }

  return runnables;
}

/**
 * Writes a single runnable to its YAML file in the backend folder.
 * The file will be named `<runnableId>.yaml`.
 *
 * Converts from API format to file format:
 * - For inline scripts: keeps type: "inline"
 * - For path-based runnables: converts { type: "path", runType: "script" } to { type: "script" }
 *   and removes schema field
 */
export function writeRunnableToBackend(
  backendPath: string,
  runnableId: string,
  runnable: any,
): void {
  let runnableToWrite = { ...runnable };

  // Convert path-based runnables from API format to file format
  if (runnable.type === "path" && runnable.runType) {
    // { type: "path", runType: "script" } -> { type: "script" }
    const { type: _type, runType, schema: _schema, ...rest } = runnable;
    runnableToWrite = {
      type: runType,
      ...rest,
    };
  }

  const filePath = path.join(backendPath, `${runnableId}.yaml`);
  writeIfChanged(filePath, yamlStringify(runnableToWrite, yamlOptions));
}

const alreadySynced: string[] = [];

async function collectAppFiles(
  localPath: string,
): Promise<Record<string, string>> {
  const files: Record<string, string> = {};

  async function readDirRecursive(dir: string, basePath: string = "/") {
    const dirEntries = await readdir(dir, { withFileTypes: true });
    for (const entry of dirEntries) {
      const fullPath = dir + entry.name;
      const relativePath = basePath + entry.name;

      if (entry.isDirectory()) {
        // Skip the runnables, node_modules, and sql_to_apply subfolders
        if (
          entry.name === APP_BACKEND_FOLDER ||
          entry.name === "node_modules" ||
          entry.name === "dist" ||
          entry.name === ".claude" ||
          entry.name === "sql_to_apply"
        ) {
          continue;
        }
        await readDirRecursive(fullPath + SEP, relativePath + "/");
      } else if (entry.isFile()) {
        // Skip generated/metadata files that shouldn't be part of the app
        if (
          entry.name === "raw_app.yaml" ||
          entry.name === "package-lock.json" ||
          entry.name === "DATATABLES.md" ||
          entry.name === "AGENTS.md" ||
          entry.name === "wmill.d.ts"
        ) {
          continue;
        }
        const content = await readFile(fullPath, "utf-8");
        files[relativePath] = content;
      }
    }
  }

  await readDirRecursive(localPath);
  return files;
}

export async function pushRawApp(
  workspace: string,
  remotePath: string,
  localPath: string,
  message?: string,
): Promise<void> {
  if (alreadySynced.includes(localPath)) {
    return;
  }
  alreadySynced.push(localPath);
  remotePath = remotePath.replaceAll(SEP, "/");
  let app: any = undefined;
  // deleting old app if it exists in raw mode
  try {
    app = await wmill.getAppByPath({
      workspace,
      path: remotePath,
    });
  } catch {
    //ignore
  }
  if (app?.["policy"]?.["execution_mode"] == "anonymous") {
    app.public = true;
  }
  // console.log(app);
  if (app) {
    app.policy = undefined;
  }

  if (!localPath.endsWith(SEP)) {
    localPath += SEP;
  }
  const appFilePath = localPath + "raw_app.yaml";
  const localApp = (await yamlParseFile(appFilePath)) as AppFile;

  // Load runnables from separate YAML files in the backend folder
  // Falls back to reading from raw_app.yaml if no separate files exist (backward compat)
  const backendPath = path.join(localPath, APP_BACKEND_FOLDER);
  const runnablesFromBackend = await loadRunnablesFromBackend(backendPath);

  let runnables: Record<string, any>;
  if (Object.keys(runnablesFromBackend).length > 0) {
    // Use runnables from separate files (new format)
    runnables = runnablesFromBackend;
    log.info(
      colors.gray(
        `Loaded ${Object.keys(runnables).length} runnables from backend folder`,
      ),
    );
  } else if (localApp.runnables) {
    // Fall back to runnables from raw_app.yaml (old format)
    runnables = localApp.runnables;
    log.info(
      colors.gray(
        `Loaded ${
          Object.keys(runnables).length
        } runnables from raw_app.yaml (legacy format)`,
      ),
    );
  } else {
    runnables = {};
  }

  replaceInlineScripts(runnables, backendPath + SEP, true);
  repopulateFields(runnables);

  // Create a temporary app object for policy generation
  const appForPolicy = { ...localApp, runnables };
  await generatingPolicy(
    appForPolicy,
    remotePath,
    localApp?.["public"] ?? false,
  );

  const files = await collectAppFiles(localPath);
  async function createBundleRaw() {
    log.info(colors.yellow.bold(`Creating raw app ${remotePath} bundle...`));
    // Detect frameworks to determine entry point
    const frameworks = detectFrameworks(localPath);
    const entryFile = frameworks.svelte || frameworks.vue
      ? "index.ts"
      : "index.tsx";
    const entryPoint = localPath + entryFile;
    return await createBundle({
      entryPoint: entryPoint,
      production: true,
      minify: true,
    });
  }
  // Build the value object, including data if present
  const value: Record<string, any> = { runnables, files };
  if (localApp.data) {
    value.data = localApp.data;
  }

  if (app) {
    // Check both metadata/runnables AND files for changes
    // Files need separate comparison because isSuperset only checks if local keys exist in remote
    const metadataUpToDate = isSuperset({ ...localApp, runnables }, app);
    const filesUpToDate = deepEqual(files, app.value?.files);
    if (metadataUpToDate && filesUpToDate) {
      log.info(colors.green(`App ${remotePath} is up to date`));
      return;
    }
    const { js, css } = await createBundleRaw();
    log.info(colors.bold.yellow(`Updating app ${remotePath}...`));
    await wmill.updateAppRaw({
      workspace,
      path: remotePath,
      formData: {
        app: {
          value,
          path: remotePath,
          summary: localApp.summary,
          policy: appForPolicy.policy,
          deployment_message: message,
          ...(localApp.custom_path
            ? { custom_path: localApp.custom_path }
            : {}),
        },
        js,
        css,
      },
    });
  } else {
    const { js, css } = await createBundleRaw();
    await wmill.createAppRaw({
      workspace,
      formData: {
        app: {
          value,
          path: remotePath,
          summary: localApp.summary,
          policy: appForPolicy.policy,
          deployment_message: message,
          ...(localApp.custom_path
            ? { custom_path: localApp.custom_path }
            : {}),
        },
        js,
        css,
      },
    });
  }
}

export async function generatingPolicy(
  app: any,
  path: string,
  publicApp: boolean,
) {
  log.info(colors.gray(`Generating fresh policy for app ${path}...`));
  try {
    app.policy = await windmillUtils.updateRawAppPolicy(
      app.runnables,
      app.policy,
    );
    app.policy.execution_mode = publicApp ? "anonymous" : "publisher";
  } catch (e) {
    log.error(colors.red(`Error generating policy for app ${path}: ${e}`));
    throw e;
  }
}

async function pushRawAppCommand(
  opts: GlobalOptions,
  filePath: string,
  remotePath: string,
) {
  if (!validatePath(remotePath)) {
    return;
  }
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  await pushRawApp(workspace.workspaceId, remotePath, filePath);
  log.info(colors.bold.underline.green("Raw app pushed"));
}
