import { GlobalOptions, isSuperset } from "../../types.ts";
import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import { Confirm } from "@cliffy/prompt/confirm";
import { Table } from "@cliffy/table";
import * as log from "../../core/log.ts";
import { sep as SEP } from "node:path";
import { stringify as yamlStringify } from "yaml";
import { yamlParseFile } from "../../utils/yaml.ts";
import { validateRequiredArgs } from "../../utils/utils.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { readFile } from "node:fs/promises";
import { mkdirSync, writeFileSync } from "node:fs";
import { buildFolderPath, getMetadataFileName, loadNonDottedPathsSetting } from "../../utils/resource_folders.ts";

import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace, validatePath } from "../../core/context.ts";
import { resolve, track_job } from "../script/script.ts";
import { defaultFlowDefinition } from "../../../bootstrap/flow_bootstrap.ts";
import { SyncOptions, mergeConfigWithConfigFile } from "../../core/conf.ts";
import { FSFSElement, elementsToMap, ignoreF } from "../sync/sync.ts";
import { Flow } from "../../../gen/types.gen.ts";
import {
  collectPathScriptPaths,
  replaceInlineScripts,
  replaceAllPathScriptsWithLocal,
} from "../../../windmill-utils-internal/src/inline-scripts/replacer.ts";
import { generateFlowLockInternal } from "./flow_metadata.ts";
import { exts } from "../script/script.ts";
import type { SyncCodebase } from "../../utils/codebase.ts";
import { listSyncCodebases } from "../../utils/codebase.ts";
import {
  createPreviewLocalScriptReader,
  resolvePreviewLocalScriptState,
} from "../../utils/local_path_scripts.ts";

export interface FlowFile {
  summary: string;
  description?: string;
  value: any;
  schema?: any;
}

function normalizeOptionalString(value: string | null | undefined): string | undefined {
  return typeof value === "string" && value.trim() === "" ? undefined : value ?? undefined;
}

function normalizeComparableContent(value: string | undefined): string | undefined {
  return value?.replaceAll("\r\n", "\n").replace(/\n$/, "");
}

async function findDivergedLocalPathScripts(
  workspaceId: string,
  scriptPaths: string[],
  opts: {
    exts: string[];
    defaultTs?: "bun" | "deno";
    codebases: SyncCodebase[];
  }
): Promise<{ changed: string[]; missing: string[] }> {
  const changed: string[] = [];
  const missing: string[] = [];

  for (const scriptPath of scriptPaths) {
    const localScript = await resolvePreviewLocalScriptState(scriptPath, opts);
    if (!localScript) {
      continue;
    }

    let remoteScript;
    try {
      remoteScript = await wmill.getScriptByPath({
        workspace: workspaceId,
        path: scriptPath,
      });
    } catch {
      missing.push(scriptPath);
      continue;
    }

    const remoteLock = normalizeOptionalString(remoteScript.lock);
    const diverged =
      normalizeComparableContent(localScript.content) !==
        normalizeComparableContent(remoteScript.content) ||
      localScript.language !== remoteScript.language ||
      (localScript.lock !== undefined &&
        normalizeComparableContent(localScript.lock) !==
          normalizeComparableContent(remoteLock)) ||
      localScript.tag !== normalizeOptionalString(remoteScript.tag) ||
      localScript.codebaseDigest !== normalizeOptionalString(remoteScript.codebase);

    if (diverged) {
      changed.push(scriptPath);
    }
  }

  return { changed, missing };
}

function warnAboutLocalPathScriptDivergence(
  divergence: { changed: string[]; missing: string[] }
): void {
  if (divergence.changed.length === 0 && divergence.missing.length === 0) {
    return;
  }

  const details: string[] = [];
  if (divergence.changed.length > 0) {
    details.push(
      `These workspace scripts differ from the deployed version:\n${divergence.changed
        .map((path) => `- ${path}`)
        .join("\n")}`
    );
  }
  if (divergence.missing.length > 0) {
    details.push(
      `These scripts do not exist in the workspace yet:\n${divergence.missing
        .map((path) => `- ${path}`)
        .join("\n")}`
    );
  }

  log.warn(
    `Using local PathScript files for flow preview.\n${details.join("\n")}\nUse --remote to preview deployed workspace scripts instead.`
  );
}

const alreadySynced: string[] = [];

export async function pushFlow(
  workspace: string,
  remotePath: string,
  localPath: string,
  message?: string
): Promise<void> {
  if (alreadySynced.includes(localPath)) {
    return;
  }
  alreadySynced.push(localPath);
  remotePath = remotePath.replaceAll(SEP, "/");
  let flow: Flow | undefined = undefined;
  try {
    flow = await wmill.getFlowByPath({
      workspace: workspace,
      path: remotePath,
    });
  } catch {
    // flow doesn't exist
  }

  if (!localPath.endsWith(SEP)) {
    localPath += SEP;
  }
  const localFlow = (await yamlParseFile(localPath + "flow.yaml")) as FlowFile;

  const fileReader = async (path: string) => await readFile(localPath + path, "utf-8");
  const missingFiles: string[] = [];
  await replaceInlineScripts(
    localFlow.value.modules,
    fileReader,
    log,
    localPath,
    SEP,
    undefined,
    missingFiles
  );
  if (localFlow.value.failure_module) {
    await replaceInlineScripts([localFlow.value.failure_module], fileReader, log, localPath, SEP, undefined, missingFiles);
  }
  if (localFlow.value.preprocessor_module) {
    await replaceInlineScripts([localFlow.value.preprocessor_module], fileReader, log, localPath, SEP, undefined, missingFiles);
  }
  if (missingFiles.length > 0) {
    throw new Error(
      `Cannot push flow: missing inline script file(s): ${missingFiles.join(", ")}. ` +
      `Ensure all !inline references point to existing files.`
    );
  }

  if (flow) {
    if (isSuperset(localFlow, flow)) {
      log.info(colors.green(`Flow ${remotePath} is up to date`));
      return;
    }
    log.info(colors.bold.yellow(`Updating flow ${remotePath}...`));
    await wmill.updateFlow({
      workspace: workspace,
      path: remotePath.replaceAll(SEP, "/"),
      requestBody: {
        path: remotePath.replaceAll(SEP, "/"),
        deployment_message: message,
        ...localFlow,
      },
    });
  } else {
    log.info(colors.bold.yellow("Creating new flow..."));
    try {
      await wmill.createFlow({
        workspace: workspace,
        requestBody: {
          path: remotePath.replaceAll(SEP, "/"),
          deployment_message: message,
          ...localFlow,
        },
      });
    } catch (e) {
      throw new Error(
        //@ts-ignore
        `Failed to create flow ${remotePath}: ${e.body ?? e.message}`
      );
    }
  }
}

type Options = GlobalOptions;

async function push(opts: Options & { message?: string }, filePath: string, remotePath: string) {
  if (!validatePath(remotePath)) {
    return;
  }
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  await pushFlow(workspace.workspaceId, remotePath, filePath, opts.message);
  log.info(colors.bold.underline.green("Flow pushed"));
}

async function list(
  opts: GlobalOptions & { showArchived?: boolean; includeDraftOnly?: boolean; json?: boolean }
) {
  if (opts.json) log.setSilent(true);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  let page = 0;
  const perPage = 10;
  const total: Flow[] = [];
  while (true) {
    const res = await wmill.listFlows({
      workspace: workspace.workspaceId,
      page,
      perPage,
      showArchived: opts.showArchived ?? false,
      includeDraftOnly: opts.includeDraftOnly ?? false,
    });
    page += 1;
    total.push(...res);
    if (res.length < perPage) {
      break;
    }
  }

  if (opts.json) {
    console.log(JSON.stringify(total));
  } else {
    new Table()
      .header(["path", "summary", "edited by"])
      .padding(2)
      .border(true)
      .body(total.map((x) => [x.path, x.summary, x.edited_by]))
      .render();
  }
}
async function get(opts: GlobalOptions & { json?: boolean }, path: string) {
  if (opts.json) log.setSilent(true);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const f = await wmill.getFlowByPath({
    workspace: workspace.workspaceId,
    path,
  });
  if (opts.json) {
    console.log(JSON.stringify(f));
  } else {
    console.log(colors.bold("Path:") + " " + f.path);
    console.log(colors.bold("Summary:") + " " + (f.summary ?? ""));
    console.log(colors.bold("Description:") + " " + (f.description ?? ""));
    console.log(colors.bold("Edited by:") + " " + (f.edited_by ?? ""));
    console.log(colors.bold("Edited at:") + " " + (f.edited_at ?? ""));
    // API response type doesn't include flow value/modules — cast needed to access them
    const modules = (f as any).value?.modules;
    if (modules && Array.isArray(modules) && modules.length > 0) {
      console.log(colors.bold("Steps:"));
      for (const mod of modules) {
        const type = mod.value?.type ?? "unknown";
        const detail = mod.value?.language ?? mod.value?.path ?? "";
        console.log(`  ${mod.id}: ${type}${detail ? " (" + detail + ")" : ""}`);
      }
    }
  }
}

async function run(
  opts: GlobalOptions & {
    data?: string;
    silent: boolean;
  },
  path: string
) {
  if (opts.silent) {
    log.setSilent(true);
  }
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const input = opts.data ? await resolve(opts.data) : {};

  // Validate required args against schema when no data provided
  if (!opts.data) {
    try {
      const flow = await wmill.getFlowByPath({
        workspace: workspace.workspaceId,
        path,
      });
      validateRequiredArgs(flow.schema as Record<string, unknown>);
    } catch (e: any) {
      if (e.message?.startsWith("Missing required")) throw e;
      log.warn(`Could not fetch schema to validate args: ${e.message}`);
    }
  }

  const id = await wmill.runFlowByPath({
    workspace: workspace.workspaceId,
    path,
    requestBody: input,
  });

  let i = 0;
  while (true) {
    const jobInfo = await wmill.getJob({
      workspace: workspace.workspaceId,
      id,
    });
    if (jobInfo.flow_status!.modules.length <= i) {
      break;
    }
    const module = jobInfo.flow_status!.modules[i];

    if (module.job) {
      if (!opts.silent) {
        log.info("====== Job " + (i + 1) + " ======");
        await track_job(workspace.workspaceId, module.job);
      }
    } else {
      if (!opts.silent) {
        log.info(module.type);
      }
      await new Promise((resolve, _) =>
        setTimeout(() => resolve(undefined), 100)
      );
      continue;
    }
    i++;
  }

  // Wait for flow completion with retry (handles race when --silent skips module tracking)
  const MAX_RETRIES = 600; // ~60 seconds at 100ms intervals
  let retries = 0;
  while (retries < MAX_RETRIES) {
    try {
      const jobInfo = await wmill.getCompletedJob({
        workspace: workspace.workspaceId,
        id,
      });

      if (!opts.silent) {
        log.info(colors.green.underline.bold("Flow ran to completion"));
        log.info("\n");
      }

      if (jobInfo.success === false) {
        process.exitCode = 1;
      }

      if (opts.silent) {
        console.log(JSON.stringify(jobInfo.result ?? {}));
      } else {
        log.info(JSON.stringify(jobInfo.result ?? {}, null, 2));
      }

      break;
    } catch {
      retries++;
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
  }
  if (retries >= MAX_RETRIES) {
    throw new Error(`Timed out waiting for flow ${id} to complete`);
  }
}

async function preview(
  opts: GlobalOptions & {
    data?: string;
    silent: boolean;
    remote?: boolean;
  } & SyncOptions,
  flowPath: string
) {
  if (opts.silent) {
    log.setSilent(true);
  }
  const useLocalPathScripts = !opts.remote;
  if (useLocalPathScripts) {
    opts = await mergeConfigWithConfigFile(opts);
  }
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const codebases = useLocalPathScripts ? listSyncCodebases(opts) : [];

  // Normalize path - ensure it's a directory path to a .flow or __flow folder
  const isFlowDir = flowPath.endsWith(".flow") || flowPath.endsWith(".flow" + SEP)
    || flowPath.endsWith("__flow") || flowPath.endsWith("__flow" + SEP);
  if (!isFlowDir) {
    // Check if it's a flow.yaml file
    if (flowPath.endsWith("flow.yaml") || flowPath.endsWith("flow.json")) {
      flowPath = flowPath.substring(0, flowPath.lastIndexOf(SEP));
    } else {
      throw new Error(
        "Flow path must be a .flow/__flow directory or a flow.yaml file"
      );
    }
  }

  if (!flowPath.endsWith(SEP)) {
    flowPath += SEP;
  }

  // Read and parse the flow definition
  const localFlow = (await yamlParseFile(flowPath + "flow.yaml")) as FlowFile;

  // Replace inline scripts with their actual content
  const fileReader = async (path: string) => await readFile(flowPath + path, "utf-8");
  await replaceInlineScripts(
    localFlow.value.modules,
    fileReader,
    log,
    flowPath,
    SEP
  );
  if (localFlow.value.failure_module) {
    await replaceInlineScripts([localFlow.value.failure_module], fileReader, log, flowPath, SEP);
  }
  if (localFlow.value.preprocessor_module) {
    await replaceInlineScripts([localFlow.value.preprocessor_module], fileReader, log, flowPath, SEP);
  }

  if (useLocalPathScripts) {
    const scriptPaths = collectPathScriptPaths(localFlow.value);
    if (scriptPaths.length > 0) {
      const divergence = await findDivergedLocalPathScripts(
        workspace.workspaceId,
        scriptPaths,
        {
          exts,
          defaultTs: opts.defaultTs,
          codebases,
        }
      );
      if (!opts.silent) {
        warnAboutLocalPathScriptDivergence(divergence);
      }
    }

    const localScriptReader = createPreviewLocalScriptReader({
      exts,
      defaultTs: opts.defaultTs,
      codebases,
    });
    await replaceAllPathScriptsWithLocal(localFlow.value, localScriptReader, log);
  }

  const input = opts.data ? await resolve(opts.data) : {};

  if (!opts.silent) {
    log.info(colors.yellow(`Running flow preview for ${flowPath}...`));
  }

  log.debug(`Flow value: ${JSON.stringify(localFlow.value, null, 2)}`);

  // Run the flow preview
  let result;
  try {
    result = await wmill.runFlowPreviewAndWaitResult({
      workspace: workspace.workspaceId,
      requestBody: {
        value: localFlow.value,
        path: flowPath.substring(0, flowPath.indexOf(".flow")).replaceAll(SEP, "/"),
        args: input,
      },
    });
  } catch (e: any) {
    if (e.body) {
      // If a failure_module ran, the body contains its result — not an error
      if (e.body.result !== undefined) {
        if (opts.silent) {
          console.log(JSON.stringify(e.body.result));
        } else {
          log.info(colors.yellow.bold("Flow failed, error handler result:"));
          log.info(JSON.stringify(e.body.result, null, 2));
        }
        process.exitCode = 1;
        return;
      }
    }
    throw e;
  }

  if (opts.silent) {
    console.log(JSON.stringify(result));
  } else {
    log.info(colors.bold.underline.green("Flow preview completed"));
    log.info(JSON.stringify(result, null, 2));
  }
}

export async function generateLocks(
  opts: GlobalOptions & {
    yes?: boolean;
    dryRun?: boolean;
  } & SyncOptions,
  folder: string | undefined
) {
  log.warn(
    colors.yellow('This command is deprecated. Use "wmill generate-metadata" instead.')
  );
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  opts = await mergeConfigWithConfigFile(opts);
  if (folder) {
    // read script metadata file
    await generateFlowLockInternal(
      folder,
      false,
      workspace,
      opts
    );
  } else {
    const ignore = await ignoreF(opts);
    const elems = Object.keys(
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
    let hasAny = false;

    for (const folder of elems) {
      const candidate = await generateFlowLockInternal(
        folder,
        true,
        workspace,
        opts
      );
      if (candidate) {
        hasAny = true;
        log.info(colors.yellow.bold(`~ ${candidate}`));
      }
    }

    if (hasAny) {
      if (opts.dryRun) {
        log.info(colors.gray("Dry run complete."));
        return;
      }
      if (
        !opts.yes &&
        !(await Confirm.prompt({
          message: "Update the locks of the inline scripts of the above flows?",
          default: true,
        }))
      ) {
        return;
      }
    } else {
      log.info(colors.green.bold("No locks to update"));
      return;
    }
    for (const folder of elems) {
      await generateFlowLockInternal(
        folder,
        false,
        workspace,
        opts
      );
    }
  }
}

export async function bootstrap(
  opts: GlobalOptions & { summary: string; description: string },
  flowPath: string
) {
  if (!validatePath(flowPath)) {
    return;
  }

  await loadNonDottedPathsSetting();

  const flowDirFullPath = buildFolderPath(flowPath, "flow");
  mkdirSync(flowDirFullPath, { recursive: true });

  const newFlowDefinition = defaultFlowDefinition();
  if (opts.summary !== undefined) {
    newFlowDefinition.summary = opts.summary;
  }
  if (opts.description !== undefined) {
    newFlowDefinition.description = opts.description;
  }

  const newFlowDefinitionYaml = yamlStringify(
    newFlowDefinition as Record<string, any>
  );

  const metadataFile = getMetadataFileName("flow", "yaml");
  const flowYamlPath = `${flowDirFullPath}/${metadataFile}`;
  writeFileSync(flowYamlPath, newFlowDefinitionYaml, { flag: "wx", encoding: "utf-8" });
}

async function history(
  opts: GlobalOptions & { json?: boolean },
  flowPath: string
) {
  if (opts.json) log.setSilent(true);
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const versions = await wmill.getFlowHistory({
    workspace: workspace.workspaceId,
    path: flowPath,
  });

  if (opts.json) {
    console.log(JSON.stringify(versions));
  } else {
    if (versions.length === 0) {
      log.info("No version history found for " + flowPath);
      return;
    }
    new Table()
      .header(["Version", "Created At", "Deployment Message"])
      .padding(2)
      .border(true)
      .body(
        versions.map((v) => [
          String(v.id),
          new Date(v.created_at).toISOString().replace("T", " ").substring(0, 19),
          v.deployment_msg ?? "-",
        ])
      )
      .render();
  }
}

async function showVersion(
  opts: GlobalOptions & { json?: boolean },
  flowPath: string,
  version: string
) {
  if (opts.json) log.setSilent(true);
  opts = await mergeConfigWithConfigFile(opts);
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const flow = await wmill.getFlowVersion({
    workspace: workspace.workspaceId,
    path: flowPath,
    version: parseInt(version, 10),
  });

  if (opts.json) {
    console.log(JSON.stringify(flow));
  } else {
    console.log(colors.bold("Path:") + " " + flow.path);
    console.log(colors.bold("Summary:") + " " + (flow.summary ?? "-"));
    console.log(colors.bold("Description:") + " " + (flow.description ?? "-"));
    console.log(colors.bold("Schema:"));
    console.log(JSON.stringify(flow.schema, null, 2));
    console.log(colors.bold("Value:"));
    console.log(JSON.stringify(flow.value, null, 2));
  }
}

const command = new Command()
  .description("flow related commands")
  .option("--show-archived", "Enable archived flows in output")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("list", "list all flows")
  .option("--show-archived", "Enable archived flows in output")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("get", "get a flow's details")
  .arguments("<path:string>")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(get as any)
  .command(
    "push",
    "push a local flow spec. This overrides any remote versions."
  )
  .arguments("<file_path:string> <remote_path:string>")
  .option("--message <message:string>", "Deployment message")
  .action(push as any)
  .command("run", "run a flow by path.")
  .arguments("<path:string>")
  .option(
    "-d --data <data:string>",
    "Inputs specified as a JSON string or a file using @<filename> or stdin using @-."
  )
  .option(
    "-s --silent",
    "Do not ouput anything other then the final output. Useful for scripting."
  )
  .action(run as any)
  .command(
    "preview",
    "preview a local flow without deploying it. Runs the flow definition from local files and uses local PathScripts by default."
  )
  .arguments("<flow_path:string>")
  .option(
    "-d --data <data:string>",
    "Inputs specified as a JSON string or a file using @<filename> or stdin using @-."
  )
  .option(
    "-s --silent",
    "Do not output anything other then the final output. Useful for scripting."
  )
  .option(
    "--remote",
    "Use deployed workspace scripts for PathScript steps instead of local files."
  )
  .action(preview as any)
  .command(
    "generate-locks",
    "re-generate the lock files of all inline scripts of all updated flows"
  )
  .arguments("[flow:file]")
  .option("--yes", "Skip confirmation prompt")
  .option("--dry-run", "Perform a dry run without making changes")
  .option(
    "-i --includes <patterns:file[]>",
    "Comma separated patterns to specify which file to take into account (among files that are compatible with windmill). Patterns can include * (any string until '/') and ** (any string)"
  )
  .option(
    "-e --excludes <patterns:file[]>",
    "Comma separated patterns to specify which file to NOT take into account."
  )
  .action(generateLocks as any)
  .command("new", "create a new empty flow")
  .arguments("<flow_path:string>")
  .option("--summary <summary:string>", "flow summary")
  .option("--description <description:string>", "flow description")
  .action(bootstrap as any)
  .command("bootstrap", "create a new empty flow (alias for new)")
  .arguments("<flow_path:string>")
  .option("--summary <summary:string>", "flow summary")
  .option("--description <description:string>", "flow description")
  .action(bootstrap as any)
  .command("history", "Show version history for a flow")
  .arguments("<path:string>")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(history as any)
  .command("show-version", "Show a specific version of a flow")
  .arguments("<path:string> <version:string>")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(showVersion as any);

export default command;
