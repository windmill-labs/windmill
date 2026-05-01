import { Command } from "@cliffy/command";
import { Confirm } from "@cliffy/prompt/confirm";
import { colors } from "@cliffy/ansi/colors";
import { sep as SEP } from "node:path";
import { GlobalOptions } from "../../types.ts";
import { SyncOptions, mergeConfigWithConfigFile } from "../../core/conf.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { requireLogin } from "../../core/auth.ts";
import * as log from "../../core/log.ts";
import {
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
      try {
        await generateScriptMetadataInternal(
          e, stubWorkspace, rehashOpts, false, true, {}, codebases, false,
        );
        counts.scripts++;
      } catch (err) {
        log.warn(`Skipping ${e}: ${err instanceof Error ? err.message : err}`);
      }
    }
  }

  if (!rehashFilter?.skipFlows) {
    for (const f of flowFolders) {
      if (!inFilter(f)) continue;
      if (rehashFilter?.missingOnly) {
        const folderNormalized = f.replaceAll(SEP, "/");
        if (skipIfExisting(folderNormalized, "__flow_hash")) continue;
      }
      try {
        await generateFlowLockInternal(f, false, stubWorkspace, rehashOpts, false, true);
        counts.flows++;
      } catch (err) {
        log.warn(`Skipping ${f}: ${err instanceof Error ? err.message : err}`);
      }
    }
  }

  if (!rehashFilter?.skipApps) {
    for (const { folder: appFolder, rawApp } of appPaths) {
      if (!inFilter(appFolder)) continue;
      if (rehashFilter?.missingOnly) {
        const folderNormalized = appFolder.replaceAll(SEP, "/");
        if (skipIfExisting(folderNormalized, "__app_hash")) continue;
      }
      try {
        await generateAppLocksInternal(appFolder, rawApp, false, stubWorkspace, rehashOpts, false, true);
        counts.apps++;
      } catch (err) {
        log.warn(`Skipping ${appFolder}: ${err instanceof Error ? err.message : err}`);
      }
    }
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

async function generateMetadata(
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

  // Process scripts
  for (const item of scripts) {
    current++;
    log.info(`${formatProgress(current)} script ${item.path}`);
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
  }

  // Process flows
  for (const item of flows) {
    current++;
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
      log.info(`${formatProgress(current)} flow   ${item.path}${scriptsInfo}`);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      errors.push({ path: item.path, error: msg });
      log.info(`${formatProgress(current)} flow   ${item.path}`);
      log.error(`  Failed: ${msg}`);
    }
  }

  // Process apps
  for (const item of apps) {
    current++;
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
      log.info(`${formatProgress(current)} app    ${item.path}${scriptsInfo}`);
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      errors.push({ path: item.path, error: msg });
      log.info(`${formatProgress(current)} app    ${item.path}`);
      log.error(`  Failed: ${msg}`);
    }
  }

  // Persist all stale workspace dep hashes (not just filtered — deps are global, not folder-scoped)
  const allStaleDeps = staleItems.filter((i) => i.type === "dependencies");
  await tree.persistDepsHashes(allStaleDeps.map((d) => d.path));

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
  .description("Generate metadata (locks, schemas) for all scripts, flows, and apps")
  .arguments("[folder:string]")
  .option("--yes", "Skip confirmation prompt")
  .option("--dry-run", "Show what would be updated without making changes")
  .option("--lock-only", "Re-generate only the lock files")
  .option("--schema-only", "Re-generate only script schemas (skips flows and apps)")
  .option("--skip-scripts", "Skip processing scripts")
  .option("--skip-flows", "Skip processing flows")
  .option("--skip-apps", "Skip processing apps")
  .option("--strict-folder-boundaries", "Only update items inside the specified folder (requires folder argument)")
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
        "Trust on-disk content; rewrite wmill-lock.yaml hashes without backend " +
        "trips or yaml/lock rewrites. Useful for bootstrapping missing lockfile " +
        "entries or recovering from older-CLI hash drift."
      )
      .arguments("[folder:string]")
      .option("--skip-scripts", "Skip processing scripts")
      .option("--skip-flows", "Skip processing flows")
      .option("--skip-apps", "Skip processing apps")
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
