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
import { extractInlineScripts as extractInlineScriptsForFlows, extractCurrentMapping } from "../../../windmill-utils-internal/src/inline-scripts/extractor.ts";
import { newPathAssigner } from "../../../windmill-utils-internal/src/path-utils/path-assigner.ts";

import { generateHash, getHeaders, writeIfChanged } from "../../utils/utils.ts";
import { exts } from "../script/script.ts";
import { FSFSElement } from "../sync/sync.ts";
import { Workspace } from "../workspace/workspace.ts";
import { FlowFile } from "./flow.ts";
import { FlowValue } from "../../../gen/types.gen.ts";
import { replaceInlineScripts } from "../../../windmill-utils-internal/src/inline-scripts/replacer.ts";
import { workspaceDependenciesLanguages } from "../../utils/script_common.ts";
import { extractNameFromFolder, getFolderSuffix, getNonDottedPaths } from "../../utils/resource_folders.ts";
import { extractRelativeImports } from "../../utils/relative_imports.ts";
import { DoubleLinkedDependencyTree } from "../../utils/dependency_tree.ts";

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
/**
 * Result of generating flow locks, including which scripts were updated
 */
export interface FlowLocksResult {
  path: string;
  updatedScripts: string[];
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
  legacyBehaviour?: boolean,
  tree?: DoubleLinkedDependencyTree
): Promise<string | FlowLocksResult | void> {
  if (folder.endsWith(SEP)) {
    folder = folder.substring(0, folder.length - 1);
  }
  const remote_path = extractNameFromFolder(folder.replaceAll(SEP, "/"), "flow");
  if (!justUpdateMetadataLock && !noStaleMessage) {
    log.info(`Generating lock for flow ${folder} at ${remote_path}`);
  }

  const flowValue = (await yamlParseFile(
    folder! + SEP + "flow.yaml"
  )) as FlowFile;

  const folderNormalized = folder.replaceAll(SEP, "/");
  const inlineScriptsForTree = extractInlineScriptsForFlows(
    structuredClone(flowValue.value.modules),
    {},
    SEP,
    opts.defaultTs
  ).filter(s => !s.is_lock);

  let filteredDeps: Record<string, string> = {};
  const conf = await readLockfile();

  if (!legacyBehaviour && tree) {
    if (dryRun) {
      const inlineScriptPaths: string[] = [];
      for (const script of inlineScriptsForTree) {
        let content = script.content;
        if (content.startsWith("!inline ")) {
          const filePath = folder + SEP + content.replace("!inline ", "");
          try {
            content = await readFile(filePath, "utf-8");
          } catch {
            continue;
          }
        }

        const treePath = folderNormalized + "/" + path.basename(script.path, path.extname(script.path));
        const language = script.language as ScriptLanguage;
        const imports = await extractRelativeImports(content, treePath, language);
        await tree.addNode(treePath, content, language, "", imports, "inline_script", folderNormalized, folder, false);
        inlineScriptPaths.push(treePath);
      }

      const hashes = await generateFlowHash({}, folder, opts.defaultTs);
      const isDirectlyStale = !(await checkifMetadataUptodate(folder, hashes[TOP_HASH], conf, TOP_HASH));

      await tree.addNode(folderNormalized, "", "bun", "", inlineScriptPaths, "flow", folderNormalized, folder, isDirectlyStale);
      return;
    }
    // Second pass: get mismatched workspace deps from tree
    filteredDeps = await filterWorkspaceDependenciesForFlow(flowValue.value as FlowValue, tree.getMismatchedWorkspaceDeps(), folder);
  } else {
    const rawWorkspaceDependencies = await getRawWorkspaceDependencies(true);
    filteredDeps = await filterWorkspaceDependenciesForFlow(flowValue.value as FlowValue, rawWorkspaceDependencies, folder);

    const hashes = await generateFlowHash(filteredDeps, folder, opts.defaultTs);
    const isDirectlyStale = !(await checkifMetadataUptodate(folder, hashes[TOP_HASH], conf, TOP_HASH));

    if (!isDirectlyStale) {
      if (!noStaleMessage) {
        log.info(
          colors.green(`Flow ${remote_path} metadata is up-to-date, skipping`)
        );
      }
      return;
    } else if (dryRun) {
      return remote_path;
    }
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


  let changedScripts: string[] = [];

  // Build mapping from on-disk file names (hash keys like "a.py") to tree paths
  // (like "folder/a.inline_script"). The tree uses extractInlineScriptsForFlows without
  // a path assigner, so paths always have .inline_script suffix, but on-disk files
  // may not (non-dotted mode).
  const fileToTreePath = new Map<string, string>();
  for (const script of inlineScriptsForTree) {
    const c = script.content;
    if (c.startsWith("!inline ")) {
      const fileName = c.replace("!inline ", "");
      const treePath = folderNormalized + "/" + path.basename(script.path, path.extname(script.path));
      fileToTreePath.set(fileName, treePath);
    }
  }

  if (!justUpdateMetadataLock) {
    const hashes = await generateFlowHash(filteredDeps, folder, opts.defaultTs);

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

    // Capture existing module-ID-to-file-path mapping before replaceInlineScripts
    // overwrites the !inline references with actual file content. This preserves
    // the original filenames when re-extracting inline scripts after lock generation.
    const currentMapping = extractCurrentMapping(
      flowValue.value.modules,
      {},
      flowValue.value.failure_module,
      flowValue.value.preprocessor_module,
    );

    // In tree mode, use the tree's staleness info (which includes transitive dependency changes)
    // to determine which scripts need relocking, instead of only content-changed ones.
    const locksToRemove = (tree && !legacyBehaviour)
      ? Object.keys(hashes).filter(k => {
          if (k === TOP_HASH) return false;
          const treePath = fileToTreePath.get(k)
            ?? (folderNormalized + "/" + path.basename(k, path.extname(k)));
          return tree.isStale(treePath);
        })
      : changedScripts;
    await replaceInlineScripts(
      flowValue.value.modules,
      fileReader,
      log,
      folder + SEP!,
      SEP,
      locksToRemove
    );
    if (flowValue.value.failure_module) {
      await replaceInlineScripts([flowValue.value.failure_module], fileReader, log, folder + SEP!, SEP, locksToRemove);
    }
    if (flowValue.value.preprocessor_module) {
      await replaceInlineScripts([flowValue.value.preprocessor_module], fileReader, log, folder + SEP!, SEP, locksToRemove);
    }

    //removeChangedLocks
    const tempScriptRefs = tree?.getTempScriptRefs(folderNormalized);
    flowValue.value = await updateFlow(
      workspace,
      flowValue.value,
      remote_path,
      filteredDeps,
      tempScriptRefs
    );

    const lockAssigner = newPathAssigner(opts.defaultTs ?? "bun", {
      skipInlineScriptSuffix: getNonDottedPaths(),
    });
    const inlineScripts = extractInlineScriptsForFlows(
      flowValue.value.modules,
      currentMapping,
      SEP,
      opts.defaultTs,
      lockAssigner
    );
    if (flowValue.value.failure_module) {
      inlineScripts.push(...extractInlineScriptsForFlows([flowValue.value.failure_module], currentMapping, SEP, opts.defaultTs, lockAssigner));
    }
    if (flowValue.value.preprocessor_module) {
      inlineScripts.push(...extractInlineScriptsForFlows([flowValue.value.preprocessor_module], currentMapping, SEP, opts.defaultTs, lockAssigner));
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

  // Non-legacy mode excludes workspace deps from hash (tracked via tree instead)
  const depsForHash = (tree && !legacyBehaviour) ? {} : filteredDeps;
  const finalHashes = await generateFlowHash(
    depsForHash,
    folder,
    opts.defaultTs
  );
  await clearGlobalLock(folder);
  for (const [path, hash] of Object.entries(finalHashes)) {
    await updateMetadataGlobalLock(folder, hash, path);
  }
  if (!noStaleMessage) {
    log.info(colors.green(`Flow ${remote_path} lockfiles updated`));
  }

  // Return the list of updated scripts (extract just the filename from the path)
  // In tree mode, use the same staleness-aware list we used for lock removal
  const relocked = (tree && !legacyBehaviour)
    ? Object.keys(finalHashes).filter(k => {
        if (k === TOP_HASH) return false;
        const treePath = fileToTreePath.get(k)
          ?? (folderNormalized + "/" + path.basename(k, path.extname(k)));
        return tree.isStale(treePath);
      })
    : changedScripts;
  const updatedScripts = relocked.map(p => {
    const parts = p.split(SEP);
    return parts[parts.length - 1].replace(/\.[^.]+$/, ""); // Remove extension
  });
  return { path: remote_path, updatedScripts };
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
          ...(tempScriptRefs && Object.keys(tempScriptRefs).length > 0
            ? { temp_script_refs: tempScriptRefs }
            : {}),
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
          ...(tempScriptRefs && Object.keys(tempScriptRefs).length > 0
            ? { temp_script_refs: tempScriptRefs }
            : {}),
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
