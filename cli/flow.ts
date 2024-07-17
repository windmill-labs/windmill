// deno-lint-ignore-file no-explicit-any
import { GlobalOptions, isSuperset } from "./types.ts";
import { Confirm, SEP, log, yamlStringify } from "./deps.ts";
import {
  colors,
  Command,
  Flow,
  FlowModule,
  FlowService,
  JobService,
  Table,
  yamlParse,
} from "./deps.ts";
import { requireLogin, resolveWorkspace, validatePath } from "./context.ts";
import { resolve, track_job } from "./script.ts";
import { defaultFlowDefinition } from "./bootstrap/flow_bootstrap.ts";
import { generateFlowLockInternal } from "./metadata.ts";
import { SyncOptions, mergeConfigWithConfigFile } from "./conf.ts";
import { FSFSElement, elementsToMap, ignoreF } from "./sync.ts";
import { readInlinePathSync } from "./utils.ts";

export interface FlowFile {
  summary: string;
  description?: string;
  value: any;
  schema?: any;
}

const alreadySynced: string[] = [];

export function replaceInlineScripts(
  modules: FlowModule[],
  localPath: string,
  removeLocks: string[] | undefined
) {
  modules.forEach((m) => {
    if (m.value.type == "rawscript") {
      const path = m.value.content.split(" ")[1];
      m.value.content = Deno.readTextFileSync(localPath + path);
      const lock = m.value.lock;
      if (removeLocks && removeLocks.includes(path)) {
        m.value.lock = undefined;
      } else if (
        lock &&
        typeof lock == "string" &&
        lock.trimStart().startsWith("!inline ")
      ) {
        const path = lock.split(" ")[1];
        try {
          m.value.lock = readInlinePathSync(localPath + path);
        } catch {
          log.error(`Lock file ${path} not found`);
        }
      }
    } else if (m.value.type == "forloopflow") {
      replaceInlineScripts(m.value.modules, localPath, removeLocks);
    } else if (m.value.type == "whileloopflow") {
      replaceInlineScripts(m.value.modules, localPath, removeLocks);
    } else if (m.value.type == "branchall") {
      m.value.branches.forEach((b) =>
        replaceInlineScripts(b.modules, localPath, removeLocks)
      );
    } else if (m.value.type == "branchone") {
      m.value.branches.forEach((b) =>
        replaceInlineScripts(b.modules, localPath, removeLocks)
      );
      replaceInlineScripts(m.value.default, localPath, removeLocks);
    }
  });
}

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
    flow = await FlowService.getFlowByPath({
      workspace: workspace,
      path: remotePath,
    });
  } catch {
    // flow doesn't exist
  }

  if (!localPath.endsWith(SEP)) {
    localPath += SEP;
  }
  const localFlowRaw = await Deno.readTextFile(localPath + "flow.yaml");
  const localFlow = yamlParse(localFlowRaw) as FlowFile;

  replaceInlineScripts(localFlow.value.modules, localPath, undefined);

  if (flow) {
    if (isSuperset(localFlow, flow)) {
      log.info(colors.green(`Flow ${remotePath} is up to date`));
      return;
    }
    log.info(colors.bold.yellow(`Updating flow ${remotePath}...`));
    await FlowService.updateFlow({
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
    await FlowService.createFlow({
      workspace: workspace,
      requestBody: {
        path: remotePath.replaceAll(SEP, "/"),
        deployment_message: message,
        ...localFlow,
      },
    });
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
  opts: GlobalOptions & { showArchived?: boolean; includeDraftOnly?: boolean }
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  let page = 0;
  const perPage = 10;
  const total: Flow[] = [];
  while (true) {
    const res = await FlowService.listFlows({
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

  new Table()
    .header(["path", "summary", "edited by"])
    .padding(2)
    .border(true)
    .body(total.map((x) => [x.path, x.summary, x.edited_by]))
    .render();
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

  const id = await JobService.runFlowByPath({
    workspace: workspace.workspaceId,
    path,
    requestBody: input,
  });

  let i = 0;
  while (true) {
    const jobInfo = await JobService.getJob({
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
  const jobInfo = await JobService.getCompletedJob({
    workspace: workspace.workspaceId,
    id,
  });
  log.info(jobInfo.result ?? {});
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
    await generateFlowLockInternal(folder, false, workspace);
  } else {
    const ignore = await ignoreF(opts);
    const elems = Object.keys(
      await elementsToMap(
        await FSFSElement(Deno.cwd(), []),
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
      const candidate = await generateFlowLockInternal(folder, true, workspace);
      if (candidate) {
        hasAny = true;
        log.info(colors.green(`+ ${candidate}`));
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
      await generateFlowLockInternal(folder, false, workspace);
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
  Deno.mkdirSync(flowDirFullPath, { recursive: false });

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
  Deno.writeTextFile(flowYamlPath, newFlowDefinitionYaml, { createNew: true });
}

const command = new Command()
  .description("flow related commands")
  .option("--show-archived", "Enable archived scripts in output")
  .action(list as any)
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
  .command("bootstrap", "create a new empty flow")
  .arguments("<flow_path:string>")
  .option("--summary <summary:string>", "script summary")
  .option("--description <description:string>", "script description")
  .action(bootstrap as any);

export default command;
