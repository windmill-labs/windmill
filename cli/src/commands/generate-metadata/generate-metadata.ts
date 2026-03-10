import { Command, colors, Confirm, log, SEP } from "../../../deps.ts";
import { GlobalOptions } from "../../types.ts";
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace } from "../../core/context.ts";
import { mergeConfigWithConfigFile, SyncOptions } from "../../core/conf.ts";
import { ignoreF, elementsToMap, FSFSElement } from "../sync/sync.ts";
import { listSyncCodebases } from "../../utils/codebase.ts";
import { isFlowPath, isAppPath } from "../../utils/resource_folders.ts";
import { exts } from "../script/script.ts";
import { inferContentTypeFromFilePath, ScriptLanguage } from "../../utils/script_common.ts";
import {
  generateScriptMetadataInternal,
  getRawWorkspaceDependencies,
  parseMetadataFile,
  readLockfile,
  checkifMetadataUptodate,
} from "../../utils/metadata.ts";
import { extractRelativeImports } from "../../utils/relative_imports.ts";
import {
  DoubleLinkedDependencyTree,
  uploadScripts,
} from "../../utils/dependency_tree.ts";
import { generateFlowLockInternal } from "../flow/flow_metadata.ts";
import { extractInlineScripts as extractInlineScriptsForFlows } from "../../../windmill-utils-internal/src/inline-scripts/extractor.ts";
import { yamlParseFile } from "../../../deps.ts";
import { FlowFile } from "../flow/flow.ts";

async function generateMetadata(
  opts: GlobalOptions & {
    yes?: boolean;
    dryRun?: boolean;
  } & SyncOptions
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  opts = await mergeConfigWithConfigFile(opts);
  const codebases = await listSyncCodebases(opts);
  const rawWorkspaceDependencies = await getRawWorkspaceDependencies();
  const ignore = await ignoreF(opts);

  // Phase 1: Collect all scripts
  const scriptElems = await elementsToMap(
    await FSFSElement(Deno.cwd(), codebases, false),
    (p, isD) => {
      return (
        (!isD && !exts.some((ext) => p.endsWith(ext))) ||
        ignore(p, isD) ||
        isFlowPath(p) ||
        isAppPath(p)
      );
    },
    false,
    {}
  );

  // Phase 2: Collect all flows
  const flowElems = Object.keys(
    await elementsToMap(
      await FSFSElement(Deno.cwd(), [], true),
      (p, isD) => {
        return (
          ignore(p, isD) ||
          (!isD && !p.endsWith(SEP + "flow.yaml") && !p.endsWith(SEP + "flow.json"))
        );
      },
      false,
      {}
    )
  ).map((x) => x.substring(0, x.lastIndexOf(SEP)));

  // Phase 3: Build dependency tree with all scripts
  const tree = new DoubleLinkedDependencyTree();
  const scriptPaths = Object.keys(scriptElems);
  const scriptPathToRemote = new Map<string, string>();

  log.info("Collecting scripts...");
  for (const scriptPath of scriptPaths) {
    const remotePath = scriptPath
      .substring(0, scriptPath.indexOf("."))
      .replaceAll(SEP, "/");
    scriptPathToRemote.set(remotePath, scriptPath);

    try {
      const content = await Deno.readTextFile(scriptPath);
      const metadataWithType = await parseMetadataFile(remotePath, undefined);
      const metadata = await Deno.readTextFile(metadataWithType.path);
      const language = inferContentTypeFromFilePath(scriptPath, opts.defaultTs);
      const imports = await extractRelativeImports(content, remotePath, language);

      await tree.addScript(remotePath, content, language, metadata, imports, rawWorkspaceDependencies);
    } catch {
      continue;
    }
  }

  // Phase 4: Add flows to tree (they import what their inline scripts import)
  log.info("Collecting flows...");
  const flowImports = new Map<string, string[]>(); // flow folder -> import paths
  for (const flowFolder of flowElems) {
    try {
      const flowValue = (await yamlParseFile(flowFolder + SEP + "flow.yaml")) as FlowFile;
      const inlineScripts = extractInlineScriptsForFlows(
        structuredClone(flowValue.value.modules),
        {},
        SEP,
        opts.defaultTs
      );

      const allImports: string[] = [];
      const relativeFolderPath = flowFolder.replaceAll(SEP, "/");
      for (const s of inlineScripts.filter((s) => !s.is_lock)) {
        const language = s.language as ScriptLanguage;
        const imports = await extractRelativeImports(s.content, relativeFolderPath, language);
        allImports.push(...imports);
      }

      if (allImports.length > 0) {
        flowImports.set(flowFolder, [...new Set(allImports)]);
        // Add flow as a node that imports these scripts
        // Using empty content/metadata since flow itself doesn't need hashing here
        await tree.addScript(flowFolder, "", "deno", "", allImports, rawWorkspaceDependencies);
      }
    } catch {
      continue;
    }
  }

  // Phase 5: Check which scripts are directly stale
  const lockfile = await readLockfile();
  const directlyStale = new Set<string>();

  for (const remotePath of tree.allPaths()) {
    const hash = tree.getStalenessHash(remotePath);
    if (hash && !(await checkifMetadataUptodate(remotePath, hash, lockfile))) {
      directlyStale.add(remotePath);
    }
  }

  // Phase 6: Propagate staleness
  tree.propagateStaleness(directlyStale);

  // Phase 7: Display stale items
  const staleScripts: string[] = [];
  const staleFlows: string[] = [];

  for (const path of tree.allPaths()) {
    if (flowImports.has(path)) {
      staleFlows.push(path);
    } else if (scriptPathToRemote.has(path)) {
      staleScripts.push(path);
    }
  }

  if (staleScripts.length === 0 && staleFlows.length === 0) {
    log.info(colors.green.bold("No metadata to update"));
    return;
  }

  log.info("Stale items to update:");
  for (const remotePath of staleScripts) {
    const language = tree.getLanguage(remotePath);
    log.info(colors.green(`+ [script] ${remotePath} (${language})`));
  }
  for (const flowFolder of staleFlows) {
    log.info(colors.yellow(`+ [flow] ${flowFolder}`));
  }

  if (opts.dryRun) {
    log.info(colors.gray("Dry run complete."));
    return;
  }

  if (
    !opts.yes &&
    !(await Confirm.prompt({
      message: "Update the metadata of the above items?",
      default: true,
    }))
  ) {
    return;
  }

  // Phase 8: Upload stale scripts
  log.info(colors.gray("Uploading scripts to temp storage..."));
  await uploadScripts(tree, workspace);

  // Phase 9: Process stale scripts
  for (const remotePath of staleScripts) {
    const originalPath = scriptPathToRemote.get(remotePath);
    if (!originalPath) continue;

    await generateScriptMetadataInternal(
      originalPath,
      workspace,
      opts,
      false,
      true,
      rawWorkspaceDependencies,
      codebases,
      false,
      tree
    );
  }

  // Phase 10: Process stale flows
  for (const flowFolder of staleFlows) {
    await generateFlowLockInternal(
      flowFolder,
      false,
      workspace,
      opts,
      false,
      true,
      tree
    );
  }

  log.info(colors.green.bold("Metadata generation complete!"));
}

const command = new Command()
  .description("Generate metadata (locks, schemas) for all scripts and flows")
  .option("--yes", "Skip confirmation prompt")
  .option("--dry-run", "Perform a dry run without making changes")
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
