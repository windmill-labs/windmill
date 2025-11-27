// deno-lint-ignore-file no-explicit-any
import path from "node:path";
import {
  SEP,
  colors,
  log,
  yamlParseFile,
  yamlStringify,
} from "../../../deps.ts";
import { GlobalOptions } from "../../types.ts";
import {
  checkifMetadataUptodate,
  blueColor,
  clearGlobalLock,
  updateMetadataGlobalLock,
  inferSchema,
  findClosestRawReqs,
} from "../../utils/metadata.ts";
import { ScriptLanguage } from "../../utils/script_common.ts";
import {
  inferContentTypeFromFilePath,
  languagesWithRawReqsSupport,
} from "../../utils/script_common.ts";
import { generateHash, getHeaders, writeIfChanged } from "../../utils/utils.ts";
import { exts, findGlobalDeps } from "../script/script.ts";
import { FSFSElement, yamlOptions } from "../sync/sync.ts";
import { Workspace } from "../workspace/workspace.ts";
import { AppFile } from "./raw_apps.ts";
import { replaceInlineScripts } from "./apps.ts";
import { getLanguageExtension, SupportedLanguage } from "../../../windmill-utils-internal/src/path-utils/path-assigner.ts";

const TOP_HASH = "__app_hash";

/**
 * Generates a hash for all inline scripts in an app directory
 */
async function generateAppHash(
  rawReqs: Record<string, string> | undefined,
  folder: string,
  defaultTs: "bun" | "deno" | undefined
): Promise<Record<string, string>> {
  const runnablesFolder = path.join(folder, "runnables");
  const hashes: Record<string, string> = {};

  try {
    const elems = await FSFSElement(runnablesFolder, [], true);
    for await (const f of elems.getChildren()) {
      if (exts.some((e) => f.path.endsWith(e))) {
        let reqs: string | undefined;
        if (rawReqs) {
          // Get language name from path
          const lang = inferContentTypeFromFilePath(f.path, defaultTs);
          // Get lock for that language
          [, reqs] =
            Object.entries(rawReqs).find(([lang2, _]) => lang == lang2) ?? [];
        }
        // Embed lock into hash
        const relativePath = f.path.replace(runnablesFolder + SEP, "");
        hashes[relativePath] = await generateHash(
          (await f.getContentText()) + (reqs ?? "")
        );
      }
    }
  } catch (error: any) {
    // If runnables folder doesn't exist, that's okay
    if (error.name !== "NotFound") {
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
  dryRun: boolean,
  workspace: Workspace,
  opts: GlobalOptions & {
    defaultTs?: "bun" | "deno";
  },
  justUpdateMetadataLock?: boolean,
  noStaleMessage?: boolean,
  useRawReqs?: boolean
): Promise<string | void> {
  if (appFolder.endsWith(SEP)) {
    appFolder = appFolder.substring(0, appFolder.length - 1);
  }

  const remote_path = appFolder.replaceAll(SEP, "/");

  if (!justUpdateMetadataLock && !noStaleMessage) {
    log.info(`Generating locks for app ${appFolder} at ${remote_path}`);
  }

  let rawReqs: Record<string, string> | undefined = undefined;
  if (useRawReqs) {
    // Find all dependency files in the workspace
    const globalDeps = await findGlobalDeps();

    // Find closest dependency files for this app
    rawReqs = {};

    languagesWithRawReqsSupport.map((lang) => {
      const dep = findClosestRawReqs(lang, appFolder, globalDeps);
      if (dep) {
        // @ts-ignore
        rawReqs[lang.language] = dep;
      }
    });
  }

  let hashes = await generateAppHash(rawReqs, appFolder, opts.defaultTs);

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

  if (useRawReqs) {
    log.warn(
      "If using local lockfiles, following redeployments from Web App will inevitably override generated lockfiles by CLI. To maintain your script's lockfiles you will need to redeploy only from CLI. (Behavior is subject to change)"
    );
    log.info(
      (await blueColor())(
        `Found raw requirements (${languagesWithRawReqsSupport
          .map((l) => l.rrFilename)
          .join("/")}) for ${appFolder}, using it`
      )
    );
  }

  // Read the app file
  const appFilePath = path.join(appFolder, "raw_app.yaml");
  const appFile = (await yamlParseFile(appFilePath)) as AppFile;

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

      const runnablesPath = path.join(appFolder, "runnables") + SEP;

      // Replace inline scripts for changed runnables
      await replaceInlineScripts(appFile.runnables, runnablesPath);

      // Update the app runnables with new locks
      appFile.runnables = await updateAppRunnables(
        workspace,
        appFile.runnables,
        remote_path,
        appFolder,
        rawReqs,
        opts.defaultTs
      );


      // Write the updated app file
      writeIfChanged(
        appFilePath,
        yamlStringify(appFile as Record<string, any>, yamlOptions)
      );
    } else {
      log.info(colors.gray(`No scripts changed in ${appFolder}`));
    }
  }

  // Regenerate hashes after updates
  hashes = await generateAppHash(rawReqs, appFolder, opts.defaultTs);
  await clearGlobalLock(appFolder);
  for (const [scriptPath, hash] of Object.entries(hashes)) {
    await updateMetadataGlobalLock(appFolder, hash, scriptPath);
  }
  log.info(colors.green(`App ${remote_path} lockfiles updated`));
}

/**
 * Updates locks for all runnables in an app, generating locks inline script by inline script
 * Also writes content and locks back to the runnables folder
 */
async function updateAppRunnables(
  workspace: Workspace,
  runnables: Record<string, any>,
  remotePath: string,
  appFolder: string,
  rawDeps?: Record<string, string>,
  defaultTs: "bun" | "deno" = "bun"
): Promise<Record<string, any>> {
  const updatedRunnables = { ...runnables };
  const runnablesFolder = path.join(appFolder, "runnables");

  // Ensure runnables folder exists
  try {
    await Deno.mkdir(runnablesFolder, { recursive: true });
  } catch {
    // Folder may already exist
  }

  for (const [runnableId, runnable] of Object.entries(runnables)) {
    // Only process inline scripts (runnableByName with inlineScript)
    if (runnable?.type !== "runnableByName" || !runnable?.inlineScript) {
      continue;
    }

    const inlineScript = runnable.inlineScript;
    const language = inlineScript.language as SupportedLanguage;
    const content = inlineScript.content;

    if (!content || !language) {
      continue;
    }

    // Skip if content is still an !inline reference (should have been replaced by replaceInlineScripts)
    if (typeof content === "string" && content.startsWith("!inline ")) {
      log.warn(
        colors.yellow(
          `Runnable ${runnableId} content is still an !inline reference, skipping`
        )
      );
      continue;
    }

    // Skip frontend scripts - they don't need locks
    if (language === "frontend") {
      continue;
    }

    // Find raw deps for this language if available
    const langRawDeps = rawDeps?.[language];

    log.info(
      colors.gray(
        `Generating lock for runnable ${runnableId} (${language})${langRawDeps ? " with raw deps" : ""}`
      )
    );

    try {
      const lock = await generateInlineScriptLock(
        workspace,
        content,
        language,
        `${remotePath}/${runnableId}`,
        langRawDeps
      );

      // Determine file extension for this language
      const ext = getLanguageExtension(language, defaultTs);
      const baseName = `${runnableId}.inline_script`;
      const contentPath = path.join(runnablesFolder, `${baseName}.${ext}`);
      const lockPath = path.join(runnablesFolder, `${baseName}.lock`);

      // Write content to file
      writeIfChanged(contentPath, content);

      // Write lock to file if it exists
      if (lock && lock !== "") {
        writeIfChanged(lockPath, lock);
      }

      // Update the runnable with !inline references (preserve existing schema)
      const inlineContentRef = `!inline ${baseName}.${ext}`;
      const inlineLockRef = lock && lock !== "" ? `!inline ${baseName}.lock` : "";

      updatedRunnables[runnableId] = {
        ...runnable,
        inlineScript: {
          ...inlineScript,
          content: inlineContentRef,
          lock: inlineLockRef,
        },
      };

      log.info(
        colors.gray(
          `  Written ${baseName}.${ext}${lock ? ` and ${baseName}.lock` : ""}`
        )
      );
    } catch (error: any) {
      log.error(
        colors.red(
          `Failed to generate lock for runnable ${runnableId}: ${error.message}`
        )
      );
      // Continue with other runnables even if one fails
    }
  }

  return updatedRunnables;
}

/**
 * Generates a lock for a single inline script using the dependencies endpoint
 */
async function generateInlineScriptLock(
  workspace: Workspace,
  content: string,
  language: string,
  scriptPath: string,
  rawDeps?: string
): Promise<string> {
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
        raw_deps: rawDeps,
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
 * Infers and updates schema for a single runnable in a raw app.
 * Used by dev server to update schema on file changes.
 *
 * @param appFolder - The folder containing the raw app
 * @param runnableFilePath - The path to the changed runnable file (relative to runnables folder)
 * @returns The runnable ID if updated, undefined if no update was needed
 */
export async function inferRunnableSchemaFromFile(
  appFolder: string,
  runnableFilePath: string
): Promise<string | undefined> {
  // Extract runnable ID from file path (e.g., "myRunnable.inline_script.ts" -> "myRunnable")
  const fileName = path.basename(runnableFilePath);

  // Skip lock files
  if (fileName.endsWith(".lock")) {
    return undefined;
  }

  // Match pattern: {runnableId}.inline_script.{ext}
  const match = fileName.match(/^(.+)\.inline_script\.[^.]+$/);
  if (!match) {
    return undefined;
  }

  const runnableId = match[1];

  // Read the app file
  const appFilePath = path.join(appFolder, "raw_app.yaml");
  const appFile = (await yamlParseFile(appFilePath)) as AppFile;

  if (!appFile.runnables?.[runnableId]) {
    log.warn(colors.yellow(`Runnable ${runnableId} not found in raw_app.yaml`));
    return undefined;
  }

  const runnable = appFile.runnables[runnableId];

  // Only process inline scripts
  if (runnable?.type !== "runnableByName" || !runnable?.inlineScript) {
    return undefined;
  }

  const inlineScript = runnable.inlineScript;
  const language = inlineScript.language as SupportedLanguage;
  // Skip frontend scripts - they don't need schema inference
  if (language === "frontend") {
    return undefined;
  }

  // Read the actual content from the file
  const fullFilePath = path.join(appFolder, "runnables", runnableFilePath);
  let content: string;
  try {
    content = await Deno.readTextFile(fullFilePath);
  } catch {
    log.warn(colors.yellow(`Could not read file: ${fullFilePath}`));
    return undefined;
  }

  // Infer schema from script content
  const currentSchema = inlineScript.schema
  const oldSchemaStr = JSON.stringify(currentSchema);

  const remotePath = appFolder.replaceAll(SEP, "/");
  try {
    const schemaResult = await inferSchema(
      language as ScriptLanguage,
      content,
      currentSchema,
      `${remotePath}/${runnableId}`
    );

    // Check if schema actually changed
    const newSchemaStr = JSON.stringify(schemaResult.schema);


    if (newSchemaStr === oldSchemaStr) {
      return undefined;
    }

    // Update the schema in the app file
    appFile.runnables[runnableId].inlineScript.schema = schemaResult.schema;

    log.info(colors.gray(`  Writing updated app file ${appFilePath}`));
    // Write the updated app file
    writeIfChanged(
      appFilePath,
      yamlStringify(appFile as Record<string, any>, yamlOptions)
    );

    log.info(colors.green(`  Updated schema for ${runnableId}`));
    return runnableId;
  } catch (schemaError: any) {
    log.warn(
      colors.yellow(
        `Failed to infer schema for ${runnableId}: ${schemaError.message}`
      )
    );
    return undefined;
  }
}
