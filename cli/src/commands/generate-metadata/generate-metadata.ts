import { Command } from "@cliffy/command";
import { Confirm } from "@cliffy/prompt/confirm";
import { colors } from "@cliffy/ansi/colors";
import { sep as SEP } from "node:path";
import { GlobalOptions } from "../../types.ts";
import { SyncOptions, mergeConfigWithConfigFile } from "../../core/conf.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { requireLogin } from "../../core/auth.ts";
import * as log from "../../core/log.ts";
import { Workspace } from "../workspace/workspace.ts";
import {
  beginLockfileBatch,
  flushLockfileBatch,
  generateScriptMetadataInternal,
  getRawWorkspaceDependencies,
  readLockfile,
} from "../../utils/metadata.ts";
import { generateFlowLockInternal, FlowLocksResult } from "../flow/flow_metadata.ts";
import { generateAppLocksInternal, AppLocksResult } from "../app/app_metadata.ts";
import {
  elementsToMap,
  FSFSElement,
  ignoreF,
} from "../sync/sync.ts";
import { exts } from "../script/script.ts";
import { isFolderResourcePathAnyFormat, isScriptModulePath, isModuleEntryPoint, scriptPathToRemotePath } from "../../utils/resource_folders.ts";
import { listSyncCodebases, SyncCodebase } from "../../utils/codebase.ts";
import {
  DoubleLinkedDependencyTree,
  uploadScripts,
  ItemType,
} from "../../utils/dependency_tree.ts";

interface StaleItem {
  type: ItemType;
  path: string;
  folder: string;
  isRawApp?: boolean;
  staleReason?: string;
}

/**
 * FS walk helpers — shared between the regular `generate-metadata` flow and
 * the `generate-metadata rehash` subcommand. Each returns the filtered list
 * of items in scope (scripts / flow folders / app items).
 */
async function walkLocalScripts(
  codebases: SyncCodebase[],
  ignore: (p: string, isD: boolean) => boolean,
): Promise<string[]> {
  const elems = await elementsToMap(
    await FSFSElement(process.cwd(), codebases, false),
    (p, isD) =>
      (!isD && !exts.some((ext) => p.endsWith(ext))) ||
      ignore(p, isD) ||
      isFolderResourcePathAnyFormat(p) ||
      (isScriptModulePath(p) && !isModuleEntryPoint(p)),
    false,
    {},
  );
  return Object.keys(elems);
}

async function walkLocalFlowFolders(
  ignore: (p: string, isD: boolean) => boolean,
): Promise<string[]> {
  const elems = await elementsToMap(
    await FSFSElement(process.cwd(), [], true),
    (p, isD) =>
      ignore(p, isD) ||
      (!isD && !p.endsWith(SEP + "flow.yaml") && !p.endsWith(SEP + "flow.json")),
    false,
    {},
  );
  return Object.keys(elems).map((x) => x.substring(0, x.lastIndexOf(SEP)));
}

async function walkLocalAppItems(
  ignore: (p: string, isD: boolean) => boolean,
): Promise<{ folder: string; rawApp: boolean }[]> {
  const elems = await elementsToMap(
    await FSFSElement(process.cwd(), [], true),
    (p, isD) =>
      ignore(p, isD) ||
      (!isD && !p.endsWith(SEP + "raw_app.yaml") && !p.endsWith(SEP + "app.yaml")),
    false,
    {},
  );
  return Object.keys(elems).map((p) => ({
    folder: p.substring(0, p.lastIndexOf(SEP)),
    rawApp: p.endsWith(SEP + "raw_app.yaml"),
  }));
}

/**
 * Build the path -> temp-storage-hash map for a preview target (script, flow,
 * or app), so a preview run resolves relative imports from not-yet-deployed
 * local content instead of the deployed scripts. Walks all local scripts so
 * transitive relative-import targets can be uploaded, then for flow/app adds
 * that item's node. The "all" target skips per-node filtering and returns refs
 * for every uploaded script — used by `wmill dev`, where the previewed item
 * changes at runtime. Degrades gracefully (returns undefined) on older
 * backends without the /raw_temp endpoints.
 */
export async function buildPreviewTempScriptRefs(
  workspace: Workspace,
  opts: GlobalOptions & SyncOptions & { defaultTs?: "bun" | "deno" },
  codebases: SyncCodebase[],
  target:
    | { kind: "script"; path: string }
    | { kind: "flow"; folder: string }
    | { kind: "app"; folder: string; rawApp: boolean }
    | { kind: "all" },
): Promise<Record<string, string> | undefined> {
  try {
    const rawWorkspaceDependencies = await getRawWorkspaceDependencies(true);
    const tree = new DoubleLinkedDependencyTree();
    tree.setWorkspaceDeps(rawWorkspaceDependencies);
    const ignore = await ignoreF(opts);

    for (const e of await walkLocalScripts(codebases, ignore)) {
      await generateScriptMetadataInternal(
        e,
        workspace,
        opts,
        true, // dryRun: only populate the tree
        true, // noStaleMessage
        rawWorkspaceDependencies,
        codebases,
        false,
        tree,
      );
    }

    let nodePath: string | undefined;
    if (target.kind === "all") {
      // No anchor node — refs are collected tree-wide below
      nodePath = undefined;
    } else if (target.kind === "script") {
      nodePath = scriptPathToRemotePath(target.path);
    } else if (target.kind === "flow") {
      const folder = target.folder.endsWith(SEP)
        ? target.folder.slice(0, -1)
        : target.folder;
      await generateFlowLockInternal(folder, true, workspace, opts, false, true, tree);
      nodePath = folder.replaceAll(SEP, "/");
    } else {
      const folder = target.folder.endsWith(SEP)
        ? target.folder.slice(0, -1)
        : target.folder;
      await generateAppLocksInternal(
        folder,
        target.rawApp,
        true,
        workspace,
        opts,
        false,
        true,
        tree,
      );
      nodePath = folder.replaceAll(SEP, "/");
    }

    tree.propagateStaleness();
    await uploadScripts(tree, workspace);
    const refs = nodePath !== undefined
      ? tree.getTempScriptRefs(nodePath)
      : tree.getAllTempScriptRefs();
    return refs && Object.keys(refs).length > 0 ? refs : undefined;
  } catch (e) {
    // Degrade gracefully (preview still runs against deployed versions) but do
    // NOT mask the real error: only the missing-/raw_temp-endpoint case is an
    // expected old-backend incompatibility — anything else is surfaced verbatim.
    const msg = e instanceof Error ? e.message : String(e);
    // Narrow: only the missing raw_temp endpoint is the expected old-backend
    // signal. A bare 404/"not found" matches far too much (module/command/
    // ENOENT "not found", "Script X not found", …) and would mislabel real
    // bugs as a backend-too-old issue.
    const isOldBackend = /raw_temp|raw_script_temp/i.test(msg);
    if (!(opts as { silent?: boolean }).silent) {
      log.warn(
        colors.yellow(
          isOldBackend
            ? `Backend does not support local-import resolution for preview ` +
                `(requires the /raw_temp endpoints); relative imports will use ` +
                `deployed script versions.`
            : `Failed to resolve local relative imports for preview: ${msg}. ` +
                `Falling back to deployed script versions.`,
        ),
      );
    }
    return undefined;
  }
}

/**
 * Categorize a flat list of file paths into scripts / flow folders / app
 * file paths. Used to derive item lists from a precomputed FS map (e.g.
 * sync pull's change-tracker output) without re-walking the filesystem.
 *
 * Caller invariant: the provided paths are expected to already be filtered
 * by the user-level ignore predicate (`ignoreF(opts)`). This function does
 * NOT re-apply ignore patterns — it only filters by file *kind* (script vs
 * flow vs app). Sync pull's localMap satisfies this since `elementsToMap`
 * was called with the same `ignoreF`.
 */
function categorizeLocalFiles(
  paths: Iterable<string>,
): { scripts: string[]; flowFolders: string[]; appPaths: string[] } {
  const scripts: string[] = [];
  const flowFolderSet = new Set<string>();
  const appPaths: string[] = [];
  for (const p of paths) {
    if (p.endsWith(SEP + "flow.yaml") || p.endsWith(SEP + "flow.json")) {
      flowFolderSet.add(p.substring(0, p.lastIndexOf(SEP)));
    } else if (
      p.endsWith(SEP + "raw_app.yaml") ||
      p.endsWith(SEP + "app.yaml")
    ) {
      appPaths.push(p);
    } else if (
      exts.some((ext) => p.endsWith(ext)) &&
      !isFolderResourcePathAnyFormat(p) &&
      !(isScriptModulePath(p) && !isModuleEntryPoint(p))
    ) {
      scripts.push(p);
    }
  }
  return { scripts, flowFolders: [...flowFolderSet], appPaths };
}

/**
 * Walks all local scripts/flows/apps (or those under `folder`) and writes
 * canonical hashes to wmill-lock.yaml from disk content. No backend round-trip,
 * no yaml/lock rewrites. Stub workspace + opts are passed through to handlers
 * since the rehash-only fast path returns before any backend call.
 */
export async function rehashOnly(
  opts: GlobalOptions & SyncOptions & { defaultTs?: "bun" | "deno" },
  folder?: string,
  rehashFilter?: {
    missingOnly?: boolean;
    localFiles?: Iterable<string>;
    skipScripts?: boolean;
    skipFlows?: boolean;
    skipApps?: boolean;
  },
): Promise<{ scripts: number; flows: number; apps: number }> {
  const codebases = await listSyncCodebases(opts);
  const ignore = await ignoreF(opts);
  const counts = { scripts: 0, flows: 0, apps: 0 };
  const folderFilter = folder
    ?.replaceAll("\\", "/")
    .replace(/^\.\//, "")
    .replace(/\/$/, "");
  const inFilter = (p: string) => {
    if (!folderFilter) return true;
    const n = p.replaceAll("\\", "/");
    return n === folderFilter || n.startsWith(folderFilter + "/");
  };

  const conf = rehashFilter?.missingOnly ? await readLockfile() : undefined;
  const isFlatKeyed = conf?.version === "v2";
  const hasEntry = (key: string, subpath?: string): boolean => {
    if (!conf?.locks) return false;
    if (isFlatKeyed) {
      const fullKey = subpath ? `${key}+${subpath}` : key;
      return (
        conf.locks[fullKey] !== undefined ||
        conf.locks["./" + fullKey] !== undefined
      );
    }
    for (const p of [key, "./" + key]) {
      const obj = conf.locks[p];
      if (obj === undefined) continue;
      if (!subpath) return true;
      if (typeof obj === "object" && obj?.[subpath] !== undefined) return true;
    }
    return false;
  };
  const skipIfExisting = (remotePath: string, subpath?: string): boolean =>
    !!rehashFilter?.missingOnly && hasEntry(remotePath, subpath);

  // Either reuse a precomputed file list from the caller (e.g. sync pull's
  // change-tracker) or do three separate FS walks here.
  let scriptPaths: string[];
  let flowFolders: string[];
  let appPaths: { folder: string; rawApp: boolean }[];

  if (rehashFilter?.localFiles) {
    const cat = categorizeLocalFiles(rehashFilter.localFiles);
    scriptPaths = cat.scripts;
    flowFolders = cat.flowFolders;
    appPaths = cat.appPaths.map((p) => ({
      folder: p.substring(0, p.lastIndexOf(SEP)),
      rawApp: p.endsWith(SEP + "raw_app.yaml"),
    }));
  } else {
    scriptPaths = await walkLocalScripts(codebases, ignore);
    flowFolders = await walkLocalFlowFolders(ignore);
    appPaths = await walkLocalAppItems(ignore);
  }

  const stubWorkspace = {} as any;
  const rehashOpts = { ...opts, rehashOnly: true } as any;

  type RehashTask =
    | { kind: "script"; scriptPath: string }
    | { kind: "flow"; folder: string }
    | { kind: "app"; folder: string; rawApp: boolean };
  const queue: RehashTask[] = [];

  if (!rehashFilter?.skipScripts) {
    for (const e of scriptPaths) {
      // Filter against the derived remote path so a folder argument like
      // `f/foo` matches both flat (`f/foo.ts`) and folder-layout
      // (`f/foo__mod/script.ts`) scripts uniformly.
      const remotePath = scriptPathToRemotePath(e);
      if (!inFilter(remotePath)) continue;
      if (rehashFilter?.missingOnly) {
        if (skipIfExisting(remotePath) || skipIfExisting(remotePath, "__script_hash")) continue;
      }
      queue.push({ kind: "script", scriptPath: e });
    }
  }

  if (!rehashFilter?.skipFlows) {
    for (const f of flowFolders) {
      if (!inFilter(f)) continue;
      if (rehashFilter?.missingOnly) {
        const folderNormalized = f.replaceAll(SEP, "/");
        if (skipIfExisting(folderNormalized, "__flow_hash")) continue;
      }
      queue.push({ kind: "flow", folder: f });
    }
  }

  if (!rehashFilter?.skipApps) {
    for (const { folder: appFolder, rawApp } of appPaths) {
      if (!inFilter(appFolder)) continue;
      if (rehashFilter?.missingOnly) {
        const folderNormalized = appFolder.replaceAll(SEP, "/");
        if (skipIfExisting(folderNormalized, "__app_hash")) continue;
      }
      queue.push({ kind: "app", folder: appFolder, rawApp });
    }
  }

  let parallelism = Number(opts.parallel ?? 1);
  if (!Number.isFinite(parallelism) || parallelism <= 0) parallelism = 1;
  if (parallelism > 1) {
    log.info(`Parallelizing ${parallelism} items at a time`);
  }

  // Buffer wmill-lock.yaml writes during the parallel phase: each task mutates
  // the shared in-memory lockfile, then we flush once.
  await beginLockfileBatch();
  try {
    const pool = new Set<Promise<void>>();
    while (queue.length > 0 || pool.size > 0) {
      while (pool.size < parallelism && queue.length > 0) {
        const task = queue.shift()!;
        const p = (async () => {
          try {
            if (task.kind === "script") {
              await generateScriptMetadataInternal(
                task.scriptPath, stubWorkspace, rehashOpts, false, true, {}, codebases, false,
              );
              counts.scripts++;
            } else if (task.kind === "flow") {
              await generateFlowLockInternal(task.folder, false, stubWorkspace, rehashOpts, false, true);
              counts.flows++;
            } else {
              await generateAppLocksInternal(task.folder, task.rawApp, false, stubWorkspace, rehashOpts, false, true);
              counts.apps++;
            }
          } catch (err) {
            const label = task.kind === "script" ? task.scriptPath : task.folder;
            log.warn(`Skipping ${label}: ${err instanceof Error ? err.message : err}`);
          }
        })();
        pool.add(p);
        p.then(() => pool.delete(p));
      }
      if (pool.size > 0) {
        await Promise.race(pool);
      }
    }
  } finally {
    await flushLockfileBatch();
  }

  if (counts.scripts + counts.flows + counts.apps > 0 || !rehashFilter?.missingOnly) {
    log.info(
      `Rehashed ${colors.bold(String(counts.scripts))} script(s), ` +
      `${colors.bold(String(counts.flows))} flow(s), ` +
      `${colors.bold(String(counts.apps))} app(s) from disk.`,
    );
  }
  return counts;
}

export async function generateMetadata(
  opts: GlobalOptions & {
    yes?: boolean;
    lockOnly?: boolean;
    schemaOnly?: boolean;
    dryRun?: boolean;
    skipScripts?: boolean;
    skipFlows?: boolean;
    skipApps?: boolean;
    strictFolderBoundaries?: boolean;
  } & SyncOptions,
  folder?: string
) {
  if (folder === "") {
    folder = undefined;
  }

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  opts = await mergeConfigWithConfigFile(opts);

  const rawWorkspaceDependencies = await getRawWorkspaceDependencies(false);
  const codebases = await listSyncCodebases(opts);
  const ignore = await ignoreF(opts);

  // --schema-only implies skipping flows and apps (they only have locks, no schemas)
  const skipScripts = opts.skipScripts ?? false;
  const skipFlows = opts.skipFlows ?? opts.schemaOnly ?? false;
  const skipApps = opts.skipApps ?? opts.schemaOnly ?? false;

  const checking: string[] = [];
  if (!skipScripts) checking.push("scripts");
  if (!skipFlows) checking.push("flows");
  if (!skipApps) checking.push("apps");

  if (checking.length === 0) {
    log.info(colors.yellow("Nothing to check (all types skipped)"));
    return;
  }

  log.info(`Checking ${checking.join(", ")}...`);

  // Build dependency tree for relative import tracking
  const tree = new DoubleLinkedDependencyTree();
  tree.setWorkspaceDeps(rawWorkspaceDependencies);

  // === Collect stale scripts ===
  if (!skipScripts) {
    for (const e of await walkLocalScripts(codebases, ignore)) {
      await generateScriptMetadataInternal(
        e,
        workspace,
        opts,
        true, // dryRun - populate tree
        true, // noStaleMessage
        rawWorkspaceDependencies,
        codebases,
        false,
        tree
      );
    }
  }

  // === Collect stale flows ===
  if (!skipFlows) {
    for (const flowFolder of await walkLocalFlowFolders(ignore)) {
      await generateFlowLockInternal(
        flowFolder,
        true, // dryRun - populate tree
        workspace,
        opts,
        false,
        true, // noStaleMessage
        tree
      );
    }
  }

  // === Collect stale apps ===
  if (!skipApps) {
    for (const { folder: appFolder, rawApp } of await walkLocalAppItems(ignore)) {
      await generateAppLocksInternal(
        appFolder,
        rawApp,
        true, // dryRun - populate tree
        workspace,
        opts,
        false,
        true, // noStaleMessage
        tree
      );
    }
  }

  // === Propagate staleness through imports ===
  tree.propagateStaleness();

  // Upload stale scripts to temp storage so the backend can resolve relative imports.
  // If this fails (e.g. backend is older and doesn't have /raw_temp endpoints),
  // degrade gracefully: locks will be generated using deployed script content only.
  try {
    await uploadScripts(tree, workspace);
  } catch (e) {
    log.warn(colors.yellow(
      `Failed to upload scripts to temp storage (backend may be too old): ${e}. ` +
      `Locks will be generated using deployed script versions only — locally modified ` +
      `relative imports may not be reflected.`
    ));
  }

  // === Populate staleItems from tree ===
  const staleItems: StaleItem[] = [];
  const seenFolders = new Set<string>();

  for (const p of tree.allPaths()) {
    const staleReason = tree.getStaleReason(p);
    if (!staleReason) continue;

    const itemType = tree.getItemType(p)!;
    const itemFolder = tree.getFolder(p)!;

    if (itemType === "dependencies") {
      staleItems.push({ type: itemType, path: p, folder: itemFolder, staleReason });
    } else if (itemType === "inline_script") {
      // Inline scripts are not listed separately — their parent flow/app is stale via propagation
      continue;
    } else if (itemType === "script") {
      const originalPath = tree.getOriginalPath(p)!;
      staleItems.push({ type: itemType, path: originalPath, folder: itemFolder, staleReason });
    } else if (!seenFolders.has(itemFolder)) {
      // Flows/Apps: one entry per folder (dedupe multiple inline scripts)
      seenFolders.add(itemFolder);
      const originalPath = tree.getOriginalPath(p)!;
      staleItems.push({ type: itemType, path: originalPath, folder: itemFolder, isRawApp: tree.getIsRawApp(p), staleReason });
    }
  }

  // === Filter by folder if specified ===
  let filteredItems = staleItems;
  if (folder) {
    // Normalize to forward slashes (Windows users may use backslashes)
    folder = folder.replaceAll("\\", "/");
    // Strip trailing slash to match deprecated flow/app handler behavior
    if (folder.endsWith("/")) {
      folder = folder.substring(0, folder.length - 1);
    }
    // Strip file extension if user passed a specific file path (e.g. f/test/script.ts)
    const folderNoExt = folder.replace(/\.[^/.]+$/, "");
    // Check if an item is inside the specified folder
    const isInsideFolder = (item: StaleItem) => {
      const normalizedFolder = item.folder.replaceAll("\\", "/");
      const normalizedPath = item.path.replaceAll("\\", "/");
      return normalizedFolder === folder || normalizedFolder.startsWith(folder + "/")
        || normalizedPath === folder || normalizedPath === folderNoExt;
    };
    const isPathInFolder = (p: string) => p.startsWith(folder + "/") || p === folder || p === folderNoExt;
    // Check if a tree path or any of its transitive deps is inside the folder
    const touchesFolder = (treePath: string) => {
      if (isPathInFolder(treePath)) return true;
      let found = false;
      tree.traverseTransitive(treePath, (importPath) => {
        if (isPathInFolder(importPath)) {
          found = true;
          return true; // stop early
        }
      });
      return found;
    };

    const isRelevant = (item: StaleItem) => {
      if (isInsideFolder(item)) return true;
      if (item.type === "dependencies") return true;
      const treePath = (item.type === "script"
        ? item.path.replace(/\.[^/.]+$/, "")
        : item.folder).replaceAll("\\", "/");
      return touchesFolder(treePath);
    };

    if (opts.strictFolderBoundaries) {
      // Strict mode: only items inside the folder
      filteredItems = staleItems.filter(isInsideFolder);

      // Warn about stale items outside the folder that would be included by default
      const excludedStale = staleItems.filter((item) => !isInsideFolder(item) && isRelevant(item) && item.type !== "dependencies");
      for (const item of excludedStale) {
        const normalizedPath = item.path.replaceAll("\\", "/");
        log.warn(colors.yellow(
          `Warning: ${normalizedPath} depends on something inside "${folder}" but is outside it — skipped due to --strict-folder-boundaries. Next generate-metadata will not detect it as stale.`
        ));
      }
    } else {
      // Default: include items inside the folder and any stale importers that transitively depend on it
      filteredItems = staleItems.filter(isRelevant);
    }
  }

  // === Show stale items and confirm ===
  if (filteredItems.length === 0) {
    log.info(colors.green("All metadata up-to-date"));
    return;
  }

  // Group items by type for display
  const scripts = filteredItems.filter((i) => i.type === "script");
  const flows = filteredItems.filter((i) => i.type === "flow");
  const apps = filteredItems.filter((i) => i.type === "app");
  const deps = filteredItems.filter((i) => i.type === "dependencies");

  log.info("");
  log.info(`Found ${colors.bold(String(filteredItems.length))} item(s) with stale metadata:`);

  const printItems = (label: string, items: StaleItem[]) => {
    if (items.length === 0) return;
    log.info(`  ${label} (${items.length}):`);
    for (const item of items) {
      const reason = item.staleReason ? colors.dim(colors.white(` — ${item.staleReason}`)) : "";
      log.info(`    ~ ${item.path}` + reason);
    }
  };

  printItems("Workspace dependencies", deps);
  printItems("Scripts", scripts);
  printItems("Flows", flows);
  printItems("Apps", apps);

  if (opts.dryRun) {
    return;
  }

  log.info("");

  const isInteractive = process.stdin.isTTY ?? false;
  if (
    !opts.yes && isInteractive &&
    !(await Confirm.prompt({
      message: "Update metadata?",
      default: true,
    }))
  ) {
    return;
  }

  log.info("");

  // === Process all stale items with progress counter ===
  const mismatchedWorkspaceDeps = tree.getMismatchedWorkspaceDeps();
  const total = filteredItems.length - deps.length;
  const maxWidth = `[${total}/${total}]`.length;
  let current = 0;

  const formatProgress = (n: number) => {
    return colors.dim(colors.white(`[${n}/${total}]`.padEnd(maxWidth, " ")));
  };

  const errors: { path: string; error: string }[] = [];

  let parallelism = Number(opts.parallel ?? 1);
  if (!Number.isFinite(parallelism) || parallelism <= 0) parallelism = 1;
  if (parallelism > 1) {
    log.info(`Parallelizing ${parallelism} items at a time`);
  }

  type Task =
    | { kind: "script"; item: StaleItem }
    | { kind: "flow"; item: StaleItem }
    | { kind: "app"; item: StaleItem };
  const queue: Task[] = [
    ...scripts.map<Task>((item) => ({ kind: "script", item })),
    ...flows.map<Task>((item) => ({ kind: "flow", item })),
    ...apps.map<Task>((item) => ({ kind: "app", item })),
  ];

  // Buffer wmill-lock.yaml writes during the parallel phase: each task mutates
  // the shared in-memory lockfile via clearGlobalLock/updateMetadataGlobalLock,
  // then we flush once. Without this buffering, two workers' read-modify-write
  // cycles would race and lose hashes.
  await beginLockfileBatch();
  try {
    const pool = new Set<Promise<void>>();
    while (queue.length > 0 || pool.size > 0) {
      while (pool.size < parallelism && queue.length > 0) {
        const task = queue.shift()!;
        const taskNumber = ++current;
        const p = (async () => {
          if (task.kind === "script") {
            const item = task.item;
            log.info(`${formatProgress(taskNumber)} script ${item.path}`);
            try {
              await generateScriptMetadataInternal(
                item.path, // originalPath with extension
                workspace,
                opts,
                false, // dryRun
                true, // noStaleMessage
                mismatchedWorkspaceDeps,
                codebases,
                false,
                tree
              );
            } catch (e) {
              const msg = e instanceof Error ? e.message : String(e);
              errors.push({ path: item.path, error: msg });
              log.error(`  Failed: ${msg}`);
            }
          } else if (task.kind === "flow") {
            const item = task.item;
            try {
              const result = await generateFlowLockInternal(
                item.folder.replaceAll("/", SEP),
                false, // dryRun
                workspace,
                opts,
                false,
                true, // noStaleMessage
                tree
              );
              const flowResult = result as FlowLocksResult | undefined;
              const scriptsInfo = flowResult?.updatedScripts?.length
                ? colors.dim(colors.white(`: ${flowResult.updatedScripts.join(", ")}`))
                : "";
              log.info(`${formatProgress(taskNumber)} flow   ${item.path}${scriptsInfo}`);
            } catch (e) {
              const msg = e instanceof Error ? e.message : String(e);
              errors.push({ path: item.path, error: msg });
              log.info(`${formatProgress(taskNumber)} flow   ${item.path}`);
              log.error(`  Failed: ${msg}`);
            }
          } else {
            const item = task.item;
            try {
              const result = await generateAppLocksInternal(
                item.folder.replaceAll("/", SEP),
                item.isRawApp!, // rawApp
                false, // dryRun
                workspace,
                opts,
                false,
                true, // noStaleMessage
                tree
              );
              const appResult = result as AppLocksResult | undefined;
              const scriptsInfo = appResult?.updatedScripts?.length
                ? colors.dim(colors.white(`: ${appResult.updatedScripts.join(", ")}`))
                : "";
              log.info(`${formatProgress(taskNumber)} app    ${item.path}${scriptsInfo}`);
            } catch (e) {
              const msg = e instanceof Error ? e.message : String(e);
              errors.push({ path: item.path, error: msg });
              log.info(`${formatProgress(taskNumber)} app    ${item.path}`);
              log.error(`  Failed: ${msg}`);
            }
          }
        })();
        pool.add(p);
        p.then(() => pool.delete(p));
      }
      if (pool.size > 0) {
        await Promise.race(pool);
      }
    }

    // Persist all stale workspace dep hashes (not just filtered — deps are global, not folder-scoped)
    const allStaleDeps = staleItems.filter((i) => i.type === "dependencies");
    await tree.persistDepsHashes(allStaleDeps.map((d) => d.path));
  } finally {
    await flushLockfileBatch();
  }

  const succeeded = total - errors.length;
  log.info("");
  if (errors.length > 0) {
    log.info(`Done. Updated ${colors.bold(String(succeeded))}/${total} item(s). ${colors.red(String(errors.length) + " failed")}:`);
    for (const { path, error } of errors) {
      log.error(`  ${path}: ${error}`);
    }
    process.exitCode = 1;
  } else {
    log.info(`Done. Updated ${colors.bold(String(total))} item(s).`);
  }
}

async function rehashCommand(
  opts: GlobalOptions & SyncOptions & {
    skipScripts?: boolean;
    skipFlows?: boolean;
    skipApps?: boolean;
  },
  folder?: string,
) {
  if (folder === "") folder = undefined;
  opts = await mergeConfigWithConfigFile(opts);
  await rehashOnly(opts, folder, {
    skipScripts: opts.skipScripts,
    skipFlows: opts.skipFlows,
    skipApps: opts.skipApps,
  });
}

const command = new Command()
  .description("Regenerate stale local locks and script schemas and refresh wmill-lock.yaml content hashes (scripts, flows, apps). Writes local files only, not a deploy. Run it after edits that add or remove imports or change a script's arguments, so the lock, the auto-generated UI schema, and wmill-lock.yaml stay in sync.")
  .arguments("[folder:string]")
  .option("--yes", "Skip confirmation prompt")
  .option("--dry-run", "Show what would be updated without making changes")
  .option("--lock-only", "Re-generate only the lock files")
  .option("--schema-only", "Re-generate only script schemas (skips flows and apps)")
  .option("--skip-scripts", "Skip processing scripts")
  .option("--skip-flows", "Skip processing flows")
  .option("--skip-apps", "Skip processing apps")
  .option("--strict-folder-boundaries", "Only update items inside the specified folder (requires folder argument)")
  .option("--parallel <n:number>", "Number of items to process in parallel")
  .option(
    "-i --includes <patterns:file[]>",
    "Comma separated patterns to specify which files to include"
  )
  .option(
    "-e --excludes <patterns:file[]>",
    "Comma separated patterns to specify which files to exclude"
  )
  .action(generateMetadata as any)
  .command(
    "rehash",
    new Command()
      .description(
        "Refresh wmill-lock.yaml content hashes from the on-disk .lock and .script.yaml without re-resolving dependencies or hitting the backend. Use when those files are already correct and only the hashes need updating: bootstrapping missing entries or recovering from hash drift."
      )
      .arguments("[folder:string]")
      .option("--skip-scripts", "Skip processing scripts")
      .option("--skip-flows", "Skip processing flows")
      .option("--skip-apps", "Skip processing apps")
      .option("--parallel <n:number>", "Number of items to process in parallel")
      .option(
        "-i --includes <patterns:file[]>",
        "Comma separated patterns to specify which files to include"
      )
      .option(
        "-e --excludes <patterns:file[]>",
        "Comma separated patterns to specify which files to exclude"
      )
      .action(rehashCommand as any),
  );

export default command;
