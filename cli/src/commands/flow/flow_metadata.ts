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
  getRawWorkspaceDependencies,
} from "../../utils/metadata.ts";
import { extractInlineScripts as extractInlineScriptsForFlows } from "../../../windmill-utils-internal/src/inline-scripts/extractor.ts";

import { generateHash, getHeaders, writeIfChanged } from "../../utils/utils.ts";
import { exts } from "../script/script.ts";
import { FSFSElement } from "../sync/sync.ts";
import { Workspace } from "../workspace/workspace.ts";
import { FlowFile } from "./flow.ts";
import { FlowValue } from "../../../gen/types.gen.ts";
import { replaceInlineScripts } from "../../../windmill-utils-internal/src/inline-scripts/replacer.ts";
import { workspaceDependenciesLanguages } from "../../utils/script_common.ts";

const TOP_HASH = "__flow_hash";
async function generateFlowHash(
  rawWorkspaceDependencies: Record<string, string>,
  folder: string,
  defaultTs: "bun" | "deno" | undefined
) {
  const elems = await FSFSElement(path.join(Deno.cwd(), folder), [], true);
  const hashes: Record<string, string> = {};
  for await (const f of elems.getChildren()) {
    if (exts.some((e) => f.path.endsWith(e))) {
      // Embed workspace dependencies into hash
      hashes[f.path] = await generateHash(
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
  const remote_path = folder
    .replaceAll(SEP, "/")
    .substring(0, folder.length - ".flow".length);
  if (!justUpdateMetadataLock && !noStaleMessage) {
    log.info(`Generating lock for flow ${folder} at ${remote_path}`);
  }

  // Always get out-of-sync workspace dependencies
  const rawWorkspaceDependencies: Record<string, string> =
    await getRawWorkspaceDependencies();
  let hashes = await generateFlowHash(
    rawWorkspaceDependencies,
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

  if (Object.keys(rawWorkspaceDependencies).length > 0) {
    log.info(
      (await blueColor())(
        `Found workspace dependencies (${workspaceDependenciesLanguages
          .map((l) => l.filename)
          .join("/")}) for ${folder}, using them`
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
      rawWorkspaceDependencies
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

  hashes = await generateFlowHash(
    rawWorkspaceDependencies,
    folder,
    opts.defaultTs
  );
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
          raw_workspace_dependencies:
            Object.keys(rawWorkspaceDependencies).length > 0
              ? rawWorkspaceDependencies
              : null,
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
