// deno-lint-ignore-file no-explicit-any
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace, validatePath } from "../../core/context.ts";
import {
  colors,
  log,
  SEP,
  windmillUtils,
  yamlParseFile,
  yamlStringify,
} from "../../../deps.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { Policy } from "../../../gen/types.gen.ts";
import path from "node:path";

import { GlobalOptions, isSuperset } from "../../types.ts";

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
  custom_path: string;
  public?: boolean;
  summary: string;
  policy: Policy;
}

/**
 * Finds the content file for a runnable by looking for files matching the runnableId.
 * Returns the file extension and content, or undefined if not found.
 */
async function findRunnableContentFile(
  backendPath: string,
  runnableId: string,
  allFiles: string[]
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
        const content = await Deno.readTextFile(
          path.join(backendPath, fileName)
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
 * Loads all runnables from separate YAML files in the backend folder.
 * Each runnable is stored in a file named `<runnableId>.yaml`.
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
  defaultTs: "bun" | "deno" = "bun"
): Promise<Record<string, any>> {
  const runnables: Record<string, any> = {};

  try {
    // First, collect all files in the backend folder
    const allFiles: string[] = [];
    for await (const entry of Deno.readDir(backendPath)) {
      if (entry.isFile) {
        allFiles.push(entry.name);
      }
    }

    // Process YAML files (runnable metadata files)
    for (const fileName of allFiles) {
      if (!fileName.endsWith(".yaml")) {
        continue;
      }

      const runnableId = fileName.replace(".yaml", "");
      const filePath = path.join(backendPath, fileName);
      const runnable = (await yamlParseFile(filePath)) as Record<string, any>;

      // If this is an inline script (type: 'inline'), derive inlineScript from files
      if (runnable?.type === "inline") {
        const contentFile = await findRunnableContentFile(
          backendPath,
          runnableId,
          allFiles
        );

        if (contentFile) {
          const language = getLanguageFromExtension(contentFile.ext, defaultTs);

          // Try to load lock file
          let lock: string | undefined;
          try {
            lock = await Deno.readTextFile(
              path.join(backendPath, `${runnableId}.lock`)
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
  } catch (error: any) {
    if (error.name !== "NotFound") {
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
  runnable: any
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
  localPath: string
): Promise<Record<string, string>> {
  const files: Record<string, string> = {};

  async function readDirRecursive(dir: string, basePath: string = "/") {
    for await (const entry of Deno.readDir(dir)) {
      const fullPath = dir + entry.name;
      const relativePath = basePath + entry.name;

      if (entry.isDirectory) {
        // Skip the runnables and node_modules subfolders
        if (
          entry.name === APP_BACKEND_FOLDER ||
          entry.name === "node_modules" ||
          entry.name === "dist" ||
          entry.name === ".claude"
        ) {
          continue;
        }
        await readDirRecursive(fullPath + SEP, relativePath + SEP);
      } else if (entry.isFile) {
        // Skip raw_app.yaml as it's metadata, not an app file
        // Skip package-lock.json as it's generated
        if (
          entry.name === "raw_app.yaml" ||
          entry.name === "package-lock.json"
        ) {
          continue;
        }
        const content = await Deno.readTextFile(fullPath);
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
  message?: string
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
        `Loaded ${Object.keys(runnables).length} runnables from backend folder`
      )
    );
  } else if (localApp.runnables) {
    // Fall back to runnables from raw_app.yaml (old format)
    runnables = localApp.runnables;
    log.info(
      colors.gray(
        `Loaded ${
          Object.keys(runnables).length
        } runnables from raw_app.yaml (legacy format)`
      )
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
    localApp?.["public"] ?? false
  );

  const files = await collectAppFiles(localPath);
  async function createBundleRaw() {
    log.info(colors.yellow.bold(`Creating raw app ${remotePath} bundle...`));
    // Detect frameworks to determine entry point
    const frameworks = detectFrameworks(localPath);
    const entryFile =
      frameworks.svelte || frameworks.vue ? "index.ts" : "index.tsx";
    const entryPoint = localPath + entryFile;
    return await createBundle({
      entryPoint: entryPoint,
      production: true,
      minify: true,
    });
  }
  if (app) {
    if (isSuperset({ ...localApp, runnables }, app)) {
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
          value: { runnables, files },
          path: remotePath,
          summary: localApp.summary,
          policy: appForPolicy.policy,
          deployment_message: message,
          custom_path: localApp.custom_path,
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
          value: { runnables, files },
          path: remotePath,
          summary: localApp.summary,
          policy: appForPolicy.policy,
          deployment_message: message,
          custom_path: localApp.custom_path,
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
  publicApp: boolean
) {
  log.info(colors.gray(`Generating fresh policy for app ${path}...`));
  try {
    app.policy = await windmillUtils.updateRawAppPolicy(
      app.runnables,
      app.policy
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
  remotePath: string
) {
  if (!validatePath(remotePath)) {
    return;
  }
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  await pushRawApp(workspace.workspaceId, remotePath, filePath);
  log.info(colors.bold.underline.green("Raw app pushed"));
}
