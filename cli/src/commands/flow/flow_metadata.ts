import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import * as path from "node:path";
import { sep as SEP } from "node:path";
import { stringify as yamlStringify } from "yaml";
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
import { extractRelativeImports } from "../../utils/relative_imports.ts";
import { DoubleLinkedDependencyTree } from "../../utils/dependency_tree.ts";
import { extractInlineScripts as extractInlineScriptsForFlows } from "../../../windmill-utils-internal/src/inline-scripts/extractor.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { newPathAssigner } from "../../../windmill-utils-internal/src/path-utils/path-assigner.ts";

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
  noStaleMessage?: boolean,
  tree?: DoubleLinkedDependencyTree
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

  // Extract inline scripts for tree population
  const inlineScripts = extractInlineScriptsForFlows(
    structuredClone(flowValue.value.modules),
    {},
    SEP,
    opts.defaultTs
  ).filter(s => !s.is_lock);

  // If tree is provided, add inline scripts to it for dependency tracking
  const folderNormalized = folder.replaceAll(SEP, "/");
  const inlineScriptTreePaths: string[] = [];
  if (tree) {
    for (const script of inlineScripts) {
      // Resolve !inline references to actual content
      let content = script.content;
      if (content.startsWith("!inline ")) {
        const filePath = folder + SEP + content.replace("!inline ", "");
        try {
          content = await Deno.readTextFile(filePath);
        } catch {
          continue;
        }
      }

      // Path for tree: flow folder + script filename (without extension)
      const treePath = folderNormalized + "/" + path.basename(script.path, path.extname(script.path));
      inlineScriptTreePaths.push(treePath);

      const language = script.language as ScriptLanguage;
      const imports = await extractRelativeImports(content, treePath, language);

      // Use empty string for metadata since inline scripts don't have separate metadata files
      await tree.addScript(treePath, content, language, "", imports, rawWorkspaceDependencies);
    }
  }

  let hashes = await generateFlowHash(
    filteredDeps,
    folder,
    opts.defaultTs
  );

  // Staleness check: use tree if provided, otherwise use existing logic
  let shouldRegenerate: boolean;
  const conf = await readLockfile();
  if (tree) {
    // Check if any inline script in this flow is stale (in tree after propagation)
    shouldRegenerate = inlineScriptTreePaths.some(p => tree.has(p));
  } else {
    shouldRegenerate = !(await checkifMetadataUptodate(folder, hashes[TOP_HASH], conf, TOP_HASH));
  }

  if (!shouldRegenerate) {
    if (!noStaleMessage) {
      log.info(
        colors.green(`Flow ${remote_path} metadata is up-to-date, skipping`)
      );
    }
    return;
  } else if (dryRun) {
    return remote_path;
  }

  if (Object.keys(filteredDeps).length > 0 && !noStaleMessage) {
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

    if (!noStaleMessage) {
      log.info(`Recomputing locks of ${changedScripts.join(", ")} in ${folder}`);
    }
    const fileReader = async (path: string) => await readFile(folder + SEP + path, "utf-8");
    await replaceInlineScripts(
      flowValue.value.modules,
      fileReader,
      log,
      folder + SEP!,
      SEP,
      changedScripts
    );
    if (flowValue.value.failure_module) {
      await replaceInlineScripts([flowValue.value.failure_module], fileReader, log, folder + SEP!, SEP, changedScripts);
    }
    if (flowValue.value.preprocessor_module) {
      await replaceInlineScripts([flowValue.value.preprocessor_module], fileReader, log, folder + SEP!, SEP, changedScripts);
    }

    // Build tempScriptRefs from tree if provided
    const tempScriptRefs = tree?.flattenAll(inlineScriptTreePaths);

    //removeChangedLocks
    flowValue.value = await updateFlow(
      workspace,
      flowValue.value,
      remote_path,
      filteredDeps,
      tempScriptRefs
    );

    const lockAssigner = newPathAssigner(opts.defaultTs ?? "bun");
    const inlineScripts = extractInlineScriptsForFlows(
      flowValue.value.modules,
      {},
      SEP,
      opts.defaultTs,
      lockAssigner
    );
    if (flowValue.value.failure_module) {
      inlineScripts.push(...extractInlineScriptsForFlows([flowValue.value.failure_module], {}, SEP, opts.defaultTs, lockAssigner));
    }
    if (flowValue.value.preprocessor_module) {
      inlineScripts.push(...extractInlineScriptsForFlows([flowValue.value.preprocessor_module], {}, SEP, opts.defaultTs, lockAssigner));
    }
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
  if (!noStaleMessage) {
    log.info(colors.green(`Flow ${remote_path} lockfiles updated`));
  }
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
  const clonedValue = structuredClone(flowValue);
  const depAssigner = newPathAssigner("bun");
  const inlineScripts = extractInlineScriptsForFlows(clonedValue.modules, {}, SEP, undefined, depAssigner);
  if (clonedValue.failure_module) {
    inlineScripts.push(...extractInlineScriptsForFlows([clonedValue.failure_module], {}, SEP, undefined, depAssigner));
  }
  if (clonedValue.preprocessor_module) {
    inlineScripts.push(...extractInlineScriptsForFlows([clonedValue.preprocessor_module], {}, SEP, undefined, depAssigner));
  }

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
  rawWorkspaceDependencies: Record<string, string>,
  tempScriptRefs?: Record<string, string>
): Promise<FlowValue | undefined> {
  let rawResponse;

  if (Object.keys(rawWorkspaceDependencies).length > 0 || (tempScriptRefs && Object.keys(tempScriptRefs).length > 0)) {
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
          raw_workspace_dependencies: Object.keys(rawWorkspaceDependencies).length > 0
            ? rawWorkspaceDependencies : null,
          temp_script_refs: tempScriptRefs && Object.keys(tempScriptRefs).length > 0
            ? tempScriptRefs : null,
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
