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
import { isFlowPath, isAppPath, isRawAppPath, isScriptModulePath, isModuleEntryPoint } from "../../utils/resource_folders.ts";
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
  } & SyncOptions,
  folder?: string
) {
  if (folder === "") {
    folder = undefined;
  }

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  opts = await mergeConfigWithConfigFile(opts);

  const rawWorkspaceDependencies = await getRawWorkspaceDependencies();
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

  log.info(colors.gray(`Checking ${checking.join(", ")}...`));

  // Build dependency tree for relative import tracking
  const tree = new DoubleLinkedDependencyTree();

  // === Collect stale scripts ===
  if (!skipScripts) {
    // TODO: run elementsToMap only once but for all runnable types.
    const scriptElems = await elementsToMap(
      await FSFSElement(process.cwd(), codebases, false),
      (p, isD) => {
        return (
          (!isD && !exts.some((ext) => p.endsWith(ext))) ||
          ignore(p, isD) ||
          isFlowPath(p) ||
          isAppPath(p) ||
          isRawAppPath(p) ||
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

  // === Populate staleItems from tree ===
  const staleItems: StaleItem[] = [];
  const seenFolders = new Set<string>();

  for (const p of tree.allPaths()) {
    const itemType = tree.getItemType(p)!;
    const itemFolder = tree.getFolder(p)!;

    // Scripts: one entry per script (use originalPath for handler)
    // Flows/Apps: one entry per folder (dedupe multiple inline scripts)
    if (itemType === "script") {
      const originalPath = tree.getOriginalPath(p)!;
      staleItems.push({ type: itemType, path: originalPath, folder: itemFolder });
    } else if (!seenFolders.has(itemFolder)) {
      seenFolders.add(itemFolder);
      const originalPath = tree.getOriginalPath(p)!;
      staleItems.push({ type: itemType, path: originalPath, folder: itemFolder, isRawApp: tree.getIsRawApp(p) });
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
    // Normalize item.folder for comparison (Windows file paths use backslashes)
    filteredItems = staleItems.filter((item) => {
      const normalizedFolder = item.folder.replaceAll("\\", "/");
      return normalizedFolder === folder || normalizedFolder.startsWith(folder + "/");
    });
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

  log.info("");
  log.info(`Found ${filteredItems.length} item(s) with stale metadata:`);

  if (scripts.length > 0) {
    log.info(colors.gray(`  Scripts (${scripts.length}):`));
    for (const item of scripts) {
      log.info(colors.yellow(`    ${item.path}`));
    }
  }
  if (flows.length > 0) {
    log.info(colors.gray(`  Flows (${flows.length}):`));
    for (const item of flows) {
      log.info(colors.yellow(`    ${item.path}`));
    }
  }
  if (apps.length > 0) {
    log.info(colors.gray(`  Apps (${apps.length}):`));
    for (const item of apps) {
      log.info(colors.yellow(`    ${item.path}`));
    }
  }

  if (opts.dryRun) {
    return;
  }

  log.info("");

  if (
    !opts.yes &&
    !(await Confirm.prompt({
      message: "Update metadata?",
      default: true,
    }))
  ) {
    return;
  }

  log.info("");

  // === Process all stale items with progress counter ===
  const total = filteredItems.length;
  const maxWidth = `[${total}/${total}]`.length;
  let current = 0;

  const formatProgress = (n: number) => {
    const bracket = `[${n}/${total}]`;
    return colors.gray(bracket.padEnd(maxWidth, " "));
  };

  // Process scripts
  for (const item of scripts) {
    current++;
    log.info(`${formatProgress(current)} script ${colors.cyan(item.path)}`);
    await generateScriptMetadataInternal(
      item.path, // originalPath with extension
      workspace,
      opts,
      false, // dryRun
      true, // noStaleMessage
      rawWorkspaceDependencies,
      codebases,
      false,
      false, // legacyBehaviour
      tree
    );
  }

  // Process flows
  for (const item of flows) {
    current++;
    const result = await generateFlowLockInternal(
      item.folder.replaceAll("/", SEP),
      false, // dryRun
      workspace,
      opts,
      false,
      true, // noStaleMessage
      false, // legacyBehaviour
      tree
    ) as FlowLocksResult | void;
    const scriptsInfo = result?.updatedScripts?.length
      ? `: ${colors.gray(result.updatedScripts.join(", "))}`
      : "";
    log.info(`${formatProgress(current)} flow   ${colors.cyan(item.path)}${scriptsInfo}`);
  }

  // Process apps
  for (const item of apps) {
    current++;
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
    ) as AppLocksResult | void;
    const scriptsInfo = result?.updatedScripts?.length
      ? `: ${colors.gray(result.updatedScripts.join(", "))}`
      : "";
    log.info(`${formatProgress(current)} app    ${colors.cyan(item.path)}${scriptsInfo}`);
  }

  log.info("");
  log.info(colors.green(`Done. Updated ${total} item(s).`));
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
