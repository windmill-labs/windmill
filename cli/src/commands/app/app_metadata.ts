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
  findClosestRawReqs,
} from "../../utils/metadata.ts";
import {
  inferContentTypeFromFilePath,
  languagesWithRawReqsSupport,
} from "../../utils/script_common.ts";
import { generateHash, getHeaders, writeIfChanged } from "../../utils/utils.ts";
import { exts, findGlobalDeps } from "../script/script.ts";
import { FSFSElement } from "../sync/sync.ts";
import { Workspace } from "../workspace/workspace.ts";
import { AppFile } from "./raw_apps.ts";
import { replaceInlineScripts } from "./apps.ts";

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
        rawReqs
      );

      // Write the updated app file
      writeIfChanged(
        appFilePath,
        yamlStringify(appFile as Record<string, any>)
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
 * Updates locks for all runnables in an app
 */
async function updateAppRunnables(
  workspace: Workspace,
  runnables: Record<string, any>,
  remotePath: string,
  rawDeps?: Record<string, string>
): Promise<Record<string, any>> {
  let rawResponse;

  if (rawDeps != undefined) {
    log.info(colors.blue("Using raw requirements for app dependencies"));

    // Generate the script locks running a dependency job in Windmill
    const extraHeaders = getHeaders();
    rawResponse = await fetch(
      `${workspace.remote}api/w/${workspace.workspaceId}/jobs/run/dependencies`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          ...extraHeaders,
        },
        body: JSON.stringify({
          path: remotePath,
          raw_code: null,
          entrypoint: null,
          args: {},
          language: "nativets",
          raw_deps: rawDeps,
        }),
      }
    );
  } else {
    // Use the standard app dependencies endpoint
    const extraHeaders = getHeaders();
    rawResponse = await fetch(
      `${workspace.remote}api/w/${workspace.workspaceId}/jobs/run/app_dependencies`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          ...extraHeaders,
        },
        body: JSON.stringify({
          runnables,
          path: remotePath,
        }),
      }
    );
  }

  if (!rawResponse.ok) {
    const text = await rawResponse.text();
    log.error(
      colors.red(
        `Error updating app runnables locks: ${rawResponse.status} ${rawResponse.statusText}\n${text}`
      )
    );
    throw new Error(`Failed to update app runnables: ${text}`);
  }

  const jobId = await rawResponse.text();
  log.info(colors.gray(`Dependency job started: ${jobId}`));

  // Poll for job completion
  const result = await waitForJob(workspace, jobId);

  if (result && result.runnables) {
    return result.runnables;
  }

  return runnables;
}

/**
 * Waits for a job to complete and returns its result
 */
async function waitForJob(
  workspace: Workspace,
  jobId: string,
  maxAttempts = 120
): Promise<any> {
  const extraHeaders = getHeaders();

  for (let i = 0; i < maxAttempts; i++) {
    const response = await fetch(
      `${workspace.remote}api/w/${workspace.workspaceId}/jobs_u/completed/get_result_maybe/${jobId}`,
      {
        headers: extraHeaders,
      }
    );

    if (!response.ok) {
      throw new Error(`Failed to check job status: ${response.statusText}`);
    }

    const result = await response.json();

    if (result.completed) {
      if (!result.success) {
        throw new Error(`Job failed: ${JSON.stringify(result.result)}`);
      }
      return result.result;
    }

    // Wait before next poll
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }

  throw new Error(`Job ${jobId} did not complete within the timeout period`);
}
