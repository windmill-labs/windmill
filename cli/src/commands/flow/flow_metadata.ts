import { colors } from "@cliffy/ansi/colors";
import * as log from "@std/log";
import * as path from "@std/path";
import { SEPARATOR as SEP } from "@std/path";
import { stringify as yamlStringify } from "@std/yaml";
import { yamlParseFile } from "../../utils/yaml.ts";
import { readFile } from "node:fs/promises";
import { GlobalOptions } from "../../types.ts";
import {
  readLockfile,
  checkifMetadataUptodate,
  blueColor,
  clearGlobalLock,
  updateMetadataGlobalLock,
  LockfileGenerationError,
  getRawWorkspaceDependencies,
  normalizeLockPath,
  filterWorkspaceDependenciesForScripts,
} from "../../utils/metadata.ts";
import { ScriptLanguage } from "../../utils/script_common.ts";
import { extractInlineScripts as extractInlineScriptsForFlows } from "../../../windmill-utils-internal/src/inline-scripts/extractor.ts";

import { generateHash, getHeaders, writeIfChanged } from "../../utils/utils.ts";
import { exts } from "../script/script.ts";
import { FSFSElement } from "../sync/sync.ts";
import { Workspace } from "../workspace/workspace.ts";
import { FlowFile } from "./flow.ts";
import { FlowValue } from "../../../gen/types.gen.ts";
import { replaceInlineScripts } from "../../../windmill-utils-internal/src/inline-scripts/replacer.ts";
import { workspaceDependenciesLanguages } from "../../utils/script_common.ts";
import { extractNameFromFolder, getFolderSuffix } from "../../utils/resource_folders.ts";

const TOP_HASH = "__flow_hash";
async function generateFlowHash(
  rawWorkspaceDependencies: Record<string, string>,
  folder: string,
  defaultTs: "bun" | "deno" | undefined
) {
  const elems = await FSFSElement(path.join(process.cwd(), folder), [], true);
  const hashes: Record<string, string> = {};
  for await (const f of elems.getChildren()) {
    if (exts.some((e) => f.path.endsWith(e))) {
      // Embed workspace dependencies into hash
      // Normalize path to ensure OS-independent hashing
      const normalizedPath = normalizeLockPath(f.path);
      hashes[normalizedPath] = await generateHash(
        (await f.getContentText()) + JSON.stringify(rawWorkspaceDependencies)
      );
    }
  }
  return { ...hashes, [TOP_HASH]: await generateHash(JSON.stringify(hashes)) };
}
export async function generateFlowLockInternal(
  folder: string,
  dryRun: boolean,
  workspace: Workspace,
  opts: GlobalOptions & {
    defaultTs?: "bun" | "deno";
  },
  justUpdateMetadataLock?: boolean,
  noStaleMessage?: boolean
): Promise<string | void> {
  if (folder.endsWith(SEP)) {
    folder = folder.substring(0, folder.length - 1);
  }
  const remote_path = extractNameFromFolder(folder.replaceAll(SEP, "/"), "flow");
  if (!justUpdateMetadataLock && !noStaleMessage) {
    log.info(`Generating lock for flow ${folder} at ${remote_path}`);
  }

  // Always get out-of-sync workspace dependencies
  const rawWorkspaceDependencies: Record<string, string> =
    await getRawWorkspaceDependencies();

  const flowValue = (await yamlParseFile(
    folder! + SEP + "flow.yaml"
  )) as FlowFile;

  // Filter workspace dependencies based on inline scripts' languages and annotations
  const filteredDeps = await filterWorkspaceDependenciesForFlow(flowValue.value as FlowValue, rawWorkspaceDependencies, folder);

  let hashes = await generateFlowHash(
    filteredDeps,
    folder,
    opts.defaultTs
  );

  const conf = await readLockfile();
  if (await checkifMetadataUptodate(folder, hashes[TOP_HASH], conf, TOP_HASH)) {
    if (!noStaleMessage) {
      log.info(
        colors.green(`Flow ${remote_path} metadata is up-to-date, skipping`)
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
          .join("/")}) for ${folder}, using them`
      )
    );
  }


  if (!justUpdateMetadataLock) {
    const changedScripts = [];
    //find hashes that do not correspond to previous hashes
    for (const [path, hash] of Object.entries(hashes)) {
      if (path == TOP_HASH) {
        continue;
      }
      if (!(await checkifMetadataUptodate(folder, hash, conf, path))) {
        changedScripts.push(path);
      }
    }

    log.info(`Recomputing locks of ${changedScripts.join(", ")} in ${folder}`);
    await replaceInlineScripts(
      flowValue.value.modules,
      async (path: string) => await readFile(folder + SEP + path, "utf-8"),
      log,
      folder + SEP!,
      SEP,
      changedScripts
    );

    //removeChangedLocks
    flowValue.value = await updateFlow(
      workspace,
      flowValue.value,
      remote_path,
      filteredDeps
    );

    const inlineScripts = extractInlineScriptsForFlows(
      flowValue.value.modules,
      {},
      SEP,
      opts.defaultTs
    );
    inlineScripts.forEach((s) => {
      writeIfChanged(process.cwd() + SEP + folder + SEP + s.path, s.content);
    });

    // Overwrite `flow.yaml` with the new lockfile references
    writeIfChanged(
      process.cwd() + SEP + folder + SEP + "flow.yaml",
      yamlStringify(flowValue as Record<string, any>)
    );
  }

  hashes = await generateFlowHash(
    filteredDeps,
    folder,
    opts.defaultTs
  );
  await clearGlobalLock(folder);
  for (const [path, hash] of Object.entries(hashes)) {
    await updateMetadataGlobalLock(folder, hash, path);
  }
  log.info(colors.green(`Flow ${remote_path} lockfiles updated`));
}

/**
 * Filters raw workspace dependencies for a flow by extracting all inline scripts,
 * filtering deps for each based on language and annotations, then computing the union.
 */
async function filterWorkspaceDependenciesForFlow(
  flowValue: FlowValue,
  rawWorkspaceDependencies: Record<string, string>,
  folder: string
): Promise<Record<string, string>> {
  const inlineScripts = extractInlineScriptsForFlows(structuredClone(flowValue.modules), {}, SEP, undefined);

  // Filter out lock files and map to common interface
  const scripts = inlineScripts
    .filter(s => !s.is_lock)
    .map(s => ({ content: s.content, language: s.language as ScriptLanguage }));

  return await filterWorkspaceDependenciesForScripts(scripts, rawWorkspaceDependencies, folder, SEP);
}

export async function updateFlow(
  workspace: Workspace,
  flow_value: FlowValue,
  remotePath: string,
  rawWorkspaceDependencies: Record<string, string>
): Promise<FlowValue | undefined> {
  let rawResponse;

  if (Object.keys(rawWorkspaceDependencies).length > 0) {
    log.info(
      colors.blue("Using raw workspace dependencies for flow dependencies")
    );

    // generate the script lock running a dependency job in Windmill and update it inplace
    const extraHeaders = getHeaders();
    rawResponse = await fetch(
      `${workspace.remote}api/w/${workspace.workspaceId}/jobs/run/flow_dependencies`,
      {
        method: "POST",
        headers: {
          Cookie: `token=${workspace.token}`,
          "Content-Type": "application/json",
          ...extraHeaders,
        },
        body: JSON.stringify({
          flow_value,
          path: remotePath,
          use_local_lockfiles: true,
          raw_workspace_dependencies: rawWorkspaceDependencies,
        }),
      }
    );
  } else {
    // Standard dependency resolution on the server
    const extraHeaders = getHeaders();
    rawResponse = await fetch(
      `${workspace.remote}api/w/${workspace.workspaceId}/jobs/run/flow_dependencies`,
      {
        method: "POST",
        headers: {
          Cookie: `token=${workspace.token}`,
          "Content-Type": "application/json",
          ...extraHeaders,
        },
        body: JSON.stringify({
          flow_value,
          path: remotePath,
        }),
      }
    );
  }

  let responseText = "reading response failed";
  try {
    const res = (await rawResponse.json()) as
      | { updated_flow_value: any }
      | { error: { message: string } }
      | undefined;
    if (rawResponse.status != 200) {
      const msg = (res as any)?.["error"]?.["message"];
      if (msg) {
        throw new LockfileGenerationError(
          `Failed to generate lockfile: ${msg}`
        );
      }
      throw new LockfileGenerationError(
        `Failed to generate lockfile: ${rawResponse.statusText}, ${responseText}`
      );
    }
    return (res as any).updated_flow_value;
  } catch (e) {
    try {
      responseText = await rawResponse.text();
    } catch {
      responseText = "";
    }
    throw new Error(
      `Failed to generate lockfile. Status was: ${rawResponse.statusText}, ${responseText}, ${e}`
    );
  }
}
