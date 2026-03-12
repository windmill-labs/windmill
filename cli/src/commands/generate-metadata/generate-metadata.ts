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
} from "../../utils/metadata.ts";
import { generateFlowLockInternal } from "../flow/flow_metadata.ts";
import { generateAppLocksInternal } from "../app/app_metadata.ts";
import {
  elementsToMap,
  FSFSElement,
  ignoreF,
} from "../sync/sync.ts";
import { exts } from "../script/script.ts";
import { isFlowPath, isAppPath } from "../../utils/resource_folders.ts";
import { listSyncCodebases } from "../../utils/codebase.ts";

interface StaleItem {
  type: "script" | "flow" | "app";
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
  } & SyncOptions
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  opts = await mergeConfigWithConfigFile(opts);

  const rawWorkspaceDependencies = await getRawWorkspaceDependencies();
  const codebases = await listSyncCodebases(opts);
  const ignore = await ignoreF(opts);

  const staleItems: StaleItem[] = [];

  log.info(colors.gray("Checking scripts, flows, apps..."));

  // === Collect stale scripts ===
  const scriptElems = await elementsToMap(
    await FSFSElement(process.cwd(), codebases, false),
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

  for (const e of Object.keys(scriptElems)) {
    const candidate = await generateScriptMetadataInternal(
      e,
      workspace,
      opts,
      true, // dryRun
      true, // noStaleMessage
      rawWorkspaceDependencies,
      codebases,
      false
    );
    if (candidate) {
      staleItems.push({ type: "script", path: candidate, folder: e });
    }
  }

  // === Collect stale flows ===
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

  for (const folder of flowElems) {
    const candidate = await generateFlowLockInternal(
      folder,
      true, // dryRun
      workspace,
      opts,
      false,
      true // noStaleMessage
    );
    if (candidate) {
      staleItems.push({ type: "flow", path: candidate, folder });
    }
  }

  // === Collect stale apps ===
  const appElems = await elementsToMap(
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

  const rawAppFolders = Object.keys(appElems)
    .filter((p) => p.endsWith(SEP + "raw_app.yaml"))
    .map((p) => p.substring(0, p.length - (SEP + "raw_app.yaml").length));

  const normalAppFolders = Object.keys(appElems)
    .filter((p) => p.endsWith(SEP + "app.yaml"))
    .map((p) => p.substring(0, p.length - (SEP + "app.yaml").length));

  for (const appFolder of rawAppFolders) {
    const candidate = await generateAppLocksInternal(
      appFolder,
      true, // rawApp
      true, // dryRun
      workspace,
      opts,
      false,
      true // noStaleMessage
    );
    if (candidate) {
      staleItems.push({ type: "app", path: candidate, folder: appFolder, isRawApp: true });
    }
  }

  for (const appFolder of normalAppFolders) {
    const candidate = await generateAppLocksInternal(
      appFolder,
      false, // rawApp
      true, // dryRun
      workspace,
      opts,
      false,
      true // noStaleMessage
    );
    if (candidate) {
      staleItems.push({ type: "app", path: candidate, folder: appFolder, isRawApp: false });
    }
  }

  // === Show stale items and confirm ===
  if (staleItems.length === 0) {
    log.info(colors.green("All metadata up-to-date"));
    return;
  }

  log.info("");
  log.info(`Found ${staleItems.length} item(s) with stale metadata:`);
  for (const item of staleItems) {
    const prefix = item.type === "script" ? "S" : item.type === "flow" ? "F" : "A";
    log.info(colors.yellow(`  [${prefix}] ${item.path}`));
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

  // === Process all stale items ===
  for (const item of staleItems) {
    if (item.type === "script") {
      await generateScriptMetadataInternal(
        item.folder,
        workspace,
        opts,
        false, // dryRun
        false, // noStaleMessage - show per-item messages
        rawWorkspaceDependencies,
        codebases,
        false
      );
    } else if (item.type === "flow") {
      await generateFlowLockInternal(
        item.folder,
        false, // dryRun
        workspace,
        opts
        // no extra params - let it print messages
      );
    } else if (item.type === "app") {
      await generateAppLocksInternal(
        item.folder,
        item.isRawApp!, // rawApp
        false, // dryRun
        workspace,
        opts,
        false,
        true // noStaleMessage - apps suppress messages (matches original)
      );
    }
  }

  log.info(colors.green("Done"));
}

const command = new Command()
  .description("Generate metadata (locks, schemas) for all scripts, flows, and apps")
  .option("--yes", "Skip confirmation prompt")
  .option("--dry-run", "Show what would be updated without making changes")
  .option("--lock-only", "Re-generate only the lock files")
  .option("--schema-only", "Re-generate only script schemas")
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
