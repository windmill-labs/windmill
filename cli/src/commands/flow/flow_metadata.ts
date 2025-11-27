import {
  SEP,
  colors,
  log,
  path,
  yamlParseFile,
  yamlStringify,
} from "../../../deps.ts";
import { GlobalOptions } from "../../types.ts";
import {
  readLockfile,
  checkifMetadataUptodate,
  blueColor,
  clearGlobalLock,
  updateMetadataGlobalLock,
  LockfileGenerationError,
  findClosestRawReqs,
} from "../../utils/metadata.ts";
import { extractInlineScripts as extractInlineScriptsForFlows } from "../../../windmill-utils-internal/src/inline-scripts/extractor.ts";

import {
  inferContentTypeFromFilePath,
  languagesWithRawReqsSupport,
} from "../../utils/script_common.ts";
import { generateHash, getHeaders, writeIfChanged } from "../../utils/utils.ts";
import { exts, findGlobalDeps } from "../script/script.ts";
import { FSFSElement } from "../sync/sync.ts";
import { Workspace } from "../workspace/workspace.ts";
import { FlowFile } from "./flow.ts";
import { FlowValue } from "../../../gen/types.gen.ts";
import { replaceInlineScripts } from "../../../windmill-utils-internal/src/inline-scripts/replacer.ts";

const TOP_HASH = "__flow_hash";
async function generateFlowHash(
  rawReqs: Record<string, string> | undefined,
  folder: string,
  defaultTs: "bun" | "deno" | undefined
) {
  const elems = await FSFSElement(path.join(Deno.cwd(), folder), [], true);
  const hashes: Record<string, string> = {};
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
      hashes[f.path] = await generateHash(
        (await f.getContentText()) + (reqs ?? "")
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
  noStaleMessage?: boolean,
  useRawReqs?: boolean
): Promise<string | void> {
  if (folder.endsWith(SEP)) {
    folder = folder.substring(0, folder.length - 1);
  }
  const remote_path = folder
    .replaceAll(SEP, "/")
    .substring(0, folder.length - ".flow".length);
  if (!justUpdateMetadataLock && !noStaleMessage) {
    log.info(`Generating lock for flow ${folder} at ${remote_path}`);
  }

  let rawReqs: Record<string, string> | undefined = undefined;
  if (useRawReqs) {
    // Find all dependency files in the workspace
    const globalDeps = await findGlobalDeps();

    // Find closest dependency files for this flow
    rawReqs = {};

    // TODO: PERF: Only include raw reqs for the languages that are in the flow
    languagesWithRawReqsSupport.map((lang) => {
      const dep = findClosestRawReqs(lang, folder, globalDeps);
      if (dep) {
        // @ts-ignore
        rawReqs[lang.language] = dep;
      }
    });
  }
  let hashes = await generateFlowHash(rawReqs, folder, opts.defaultTs);

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

  if (useRawReqs) {
    log.warn(
      "If using local lockfiles, following redeployments from Web App will inevitably override generated lockfiles by CLI. To maintain your script's lockfiles you will need to redeploy only from CLI. (Behavior is subject to change)"
    );
    log.info(
      (await blueColor())(
        `Found raw requirements (${languagesWithRawReqsSupport
          .map((l) => l.rrFilename)
          .join("/")}) for ${folder}, using it`
      )
    );
  }

  const flowValue = (await yamlParseFile(
    folder! + SEP + "flow.yaml"
  )) as FlowFile;

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
      async (path: string) => await Deno.readTextFile(folder + SEP + path),
      log,
      folder + SEP!,
      SEP,
      changedScripts
      // (path: string, newPath: string) => Deno.renameSync(path, newPath),
      // (path: string) => Deno.removeSync(path)
    );

    //removeChangedLocks
    flowValue.value = await updateFlow(
      workspace,
      flowValue.value,
      remote_path,
      rawReqs
    );

    const inlineScripts = extractInlineScriptsForFlows(
      flowValue.value.modules,
      {},
      SEP,
      opts.defaultTs
    );
    inlineScripts.forEach((s) => {
      writeIfChanged(Deno.cwd() + SEP + folder + SEP + s.path, s.content);
    });

    // Overwrite `flow.yaml` with the new lockfile references
    writeIfChanged(
      Deno.cwd() + SEP + folder + SEP + "flow.yaml",
      yamlStringify(flowValue as Record<string, any>)
    );
  }

  hashes = await generateFlowHash(rawReqs, folder, opts.defaultTs);
  await clearGlobalLock(folder);
  for (const [path, hash] of Object.entries(hashes)) {
    await updateMetadataGlobalLock(folder, hash, path);
  }
  log.info(colors.green(`Flow ${remote_path} lockfiles updated`));
}

export async function updateFlow(
  workspace: Workspace,
  flow_value: FlowValue,
  remotePath: string,
  rawDeps?: Record<string, string>
): Promise<FlowValue | undefined> {
  let rawResponse;

  if (rawDeps != undefined) {
    log.info(colors.blue("Using raw requirements for flow dependencies"));

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
          raw_deps: rawDeps,
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
