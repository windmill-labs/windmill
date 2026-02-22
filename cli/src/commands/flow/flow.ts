import { GlobalOptions, isSuperset } from "../../types.ts";
import { colors } from "@cliffy/ansi/colors";
import { Command } from "@cliffy/command";
import { Confirm } from "@cliffy/prompt/confirm";
import { Table } from "@cliffy/table";
import * as log from "../../core/log.ts";
import { sep as SEP } from "node:path";
import { stringify as yamlStringify } from "yaml";
import { yamlParseFile } from "../../utils/yaml.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { readFile } from "node:fs/promises";
import { mkdirSync, writeFileSync } from "node:fs";

import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace, validatePath } from "../../core/context.ts";
import { resolve, track_job } from "../script/script.ts";
import { defaultFlowDefinition } from "../../../bootstrap/flow_bootstrap.ts";
import { SyncOptions, mergeConfigWithConfigFile } from "../../core/conf.ts";
import { FSFSElement, elementsToMap, ignoreF } from "../sync/sync.ts";
import { Flow } from "../../../gen/types.gen.ts";
import { replaceInlineScripts } from "../../../windmill-utils-internal/src/inline-scripts/replacer.ts";
import { generateFlowLockInternal } from "./flow_metadata.ts";

export interface FlowFile {
  summary: string;
  description?: string;
  value: any;
  schema?: any;
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

  await replaceInlineScripts(
    localFlow.value.modules,
    async (path: string) => await readFile(localPath + path, "utf-8"),
    log,
    localPath,
    SEP
  );

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

async function push(opts: Options, filePath: string, remotePath: string) {
  if (!validatePath(remotePath)) {
    return;
  }
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  await pushFlow(workspace.workspaceId, remotePath, filePath);
  log.info(colors.bold.underline.green("Flow pushed"));
}

async function list(
  opts: GlobalOptions & { showArchived?: boolean; includeDraftOnly?: boolean; json?: boolean }
) {
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
  }
}

async function run(
  opts: GlobalOptions & {
    data?: string;
    silent: boolean;
  },
  path: string
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const input = opts.data ? await resolve(opts.data) : {};

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

  if (!opts.silent) {
    log.info(colors.green.underline.bold("Flow ran to completion"));
    log.info("\n");
  }
  const jobInfo = await wmill.getCompletedJob({
    workspace: workspace.workspaceId,
    id,
  });
  log.info(JSON.stringify(jobInfo.result ?? {}, null, 2));
}

async function preview(
  opts: GlobalOptions & {
    data?: string;
    silent: boolean;
  },
  flowPath: string
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  // Normalize path - ensure it's a directory path to a .flow folder
  if (!flowPath.endsWith(".flow") && !flowPath.endsWith(".flow" + SEP)) {
    // Check if it's a flow.yaml file
    if (flowPath.endsWith("flow.yaml") || flowPath.endsWith("flow.json")) {
      flowPath = flowPath.substring(0, flowPath.lastIndexOf(SEP));
    } else {
      throw new Error(
        "Flow path must be a .flow directory or a flow.yaml file"
      );
    }
  }

  if (!flowPath.endsWith(SEP)) {
    flowPath += SEP;
  }

  // Read and parse the flow definition
  const localFlow = (await yamlParseFile(flowPath + "flow.yaml")) as FlowFile;

  // Replace inline scripts with their actual content
  await replaceInlineScripts(
    localFlow.value.modules,
    async (path: string) => await readFile(flowPath + path, "utf-8"),
    log,
    flowPath,
    SEP
  );

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
      log.error(`Flow preview failed: ${JSON.stringify(e.body)}`);
    }
    throw e;
  }

  if (opts.silent) {
    console.log(JSON.stringify(result, null, 2));
  } else {
    log.info(colors.bold.underline.green("Flow preview completed"));
    log.info(JSON.stringify(result, null, 2));
  }
}

async function generateLocks(
  opts: GlobalOptions & {
    yes?: boolean;
  } & SyncOptions,
  folder: string | undefined
) {
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

export function bootstrap(
  opts: GlobalOptions & { summary: string; description: string },
  flowPath: string
) {
  if (!validatePath(flowPath)) {
    return;
  }

  const flowDirFullPath = `${flowPath}.flow`;
  mkdirSync(flowDirFullPath, { recursive: false });

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

  const flowYamlPath = `${flowDirFullPath}/flow.yaml`;
  writeFileSync(flowYamlPath, newFlowDefinitionYaml, { flag: "wx", encoding: "utf-8" });
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
    "preview a local flow without deploying it. Runs the flow definition from local files."
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
  .action(preview as any)
  .command(
    "generate-locks",
    "re-generate the lock files of all inline scripts of all updated flows"
  )
  .arguments("[flow:file]")
  .option("--yes", "Skip confirmation prompt")
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
  .action(bootstrap as any);

export default command;
