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
  checkifMetadataUptodate,
} from "../../utils/metadata.ts";
import { generateFlowLockInternal, FlowLocksResult } from "../flow/flow_metadata.ts";
import { generateAppLocksInternal, getAppFolders, AppLocksResult } from "../app/app_metadata.ts";
import {
  elementsToMap,
  FSFSElement,
  ignoreF,
} from "../sync/sync.ts";
import { exts } from "../script/script.ts";
import { isFolderResourcePathAnyFormat, isScriptModulePath, isModuleEntryPoint } from "../../utils/resource_folders.ts";
import { listSyncCodebases } from "../../utils/codebase.ts";
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
    // TODO: run elementsToMap only once but for all runnable types.
    const scriptElems = await elementsToMap(
      await FSFSElement(process.cwd(), codebases, false),
      (p, isD) => {
        return (
          (!isD && !exts.some((ext) => p.endsWith(ext))) ||
          ignore(p, isD) ||
          isFolderResourcePathAnyFormat(p) ||
          (isScriptModulePath(p) && !isModuleEntryPoint(p))
        );
      },
      false,
      {}
    );

    for (const e of Object.keys(scriptElems)) {
      await generateScriptMetadataInternal(
        e,
        workspace,
        opts,
        true, // dryRun - populate tree
        true, // noStaleMessage
        rawWorkspaceDependencies,
        codebases,
        false,
        false, // legacyBehaviour
        tree
      );
    }
  }

  // === Collect stale flows ===
  if (!skipFlows) {
    const flowElems = Object.keys(
      await elementsToMap(
        await FSFSElement(process.cwd(), [], true),
        (p, isD) => {
          return (
            ignore(p, isD) ||
            (!isD &&
              !p.endsWith(SEP + "flow.yaml") &&
              !p.endsWith(SEP + "flow.json"))
          );
        },
        false,
        {}
      )
    ).map((x) => x.substring(0, x.lastIndexOf(SEP)));

    for (const flowFolder of flowElems) {
      await generateFlowLockInternal(
        flowFolder,
        true, // dryRun - populate tree
        workspace,
        opts,
        false,
        true, // noStaleMessage
        false, // legacyBehaviour
        tree
      );
    }
  }

  // === Collect stale apps ===
  if (!skipApps) {
    const elems = await elementsToMap(
      await FSFSElement(process.cwd(), [], true),
      (p, isD) => {
        return (
          ignore(p, isD) ||
          (!isD &&
            !p.endsWith(SEP + "raw_app.yaml") &&
            !p.endsWith(SEP + "app.yaml"))
        );
      },
      false,
      {}
    );

    const rawAppFolders = getAppFolders(elems, "raw_app.yaml");
    const appFolders = getAppFolders(elems, "app.yaml");

    for (const appFolder of rawAppFolders) {
      await generateAppLocksInternal(
        appFolder,
        true, // rawApp
        true, // dryRun - populate tree
        workspace,
        opts,
        false,
        true, // noStaleMessage
        false, // legacyBehaviour
        tree
      );
    }

    for (const appFolder of appFolders) {
      await generateAppLocksInternal(
        appFolder,
        false, // rawApp
        true, // dryRun - populate tree
        workspace,
        opts,
        false,
        true, // noStaleMessage
        false, // legacyBehaviour
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
        false, // legacyBehaviour
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
        false, // legacyBehaviour
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
        false, // legacyBehaviour
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
  .action(generateMetadata as any);

export default command;
