// deno-lint-ignore-file no-explicit-any
import {
  GlobalOptions,
  isSuperset,
  parseFromFile,
  removeType,
} from "./types.ts";
import {
  colors,
  Command,
  Flow,
  FlowService,
  JobService,
  Table,
} from "./deps.ts";
import { requireLogin, resolveWorkspace, validatePath } from "./context.ts";
import { resolve, track_job } from "./script.ts";

export interface FlowFile {
  summary: string;
  description?: string;
  value: any;
  schema?: any;
}

export async function pushFlow(
  workspace: string,
  remotePath: string,
  flow: Flow | FlowFile | undefined,
  localFlow: FlowFile
): Promise<void> {
  remotePath = removeType(remotePath, "flow");

  if (flow) {
    if (isSuperset(localFlow, flow)) {
      return;
    }
    await FlowService.updateFlow({
      workspace: workspace,
      path: remotePath,
      requestBody: {
        path: remotePath,
        ...localFlow,
      },
    });
  } else {
    console.log(colors.bold.yellow("Creating new flow..."));
    await FlowService.createFlow({
      workspace: workspace,
      requestBody: {
        path: remotePath,
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
  let flow: Flow | undefined = undefined;
  try {
    flow = await FlowService.getFlowByPath({
      workspace: workspace.workspaceId,
      path: remotePath,
    });
  } catch {
    // flow doesn't exist
  }
  await pushFlow(
    workspace.workspaceId,
    remotePath,
    flow,
    parseFromFile(filePath)
  );
  console.log(colors.bold.underline.green("Flow pushed"));
}

async function list(opts: GlobalOptions & { showArchived?: boolean }) {
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
        console.log("====== Job " + (i + 1) + " ======");
        await track_job(workspace.workspaceId, module.job);
      }
    } else {
      console.log(module.type);
      await new Promise((resolve, _) =>
        setTimeout(() => resolve(undefined), 100)
      );
      continue;
    }
    i++;
  }

  if (!opts.silent) {
    console.log(colors.green.underline.bold("Flow ran to completion"));
    console.log();
  }
  const jobInfo = await JobService.getCompletedJob({
    workspace: workspace.workspaceId,
    id,
  });
  console.log(jobInfo.result ?? {});
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
  .action(run as any);

export default command;
