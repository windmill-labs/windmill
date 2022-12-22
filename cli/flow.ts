// deno-lint-ignore-file no-explicit-any
import { GlobalOptions } from "./types.ts";
import {
  colors,
  Command,
  Flow,
  FlowService,
  JobService,
  OpenFlow,
  Table,
} from "./deps.ts";
import { requireLogin, resolveWorkspace, validatePath } from "./context.ts";
import { resolve, track_job } from "./script.ts";

type Options = GlobalOptions;

async function push(opts: Options, filePath: string, remotePath: string) {
  if (!await validatePath(opts, remotePath)) {
    return;
  }
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  await pushFlow(filePath, workspace.remote, remotePath);
  console.log(colors.bold.underline.green("Flow successfully pushed"));
}

export async function pushFlow(
  filePath: string,
  workspace: string,
  remotePath: string,
) {
  const data: OpenFlow = JSON.parse(await Deno.readTextFile(filePath));
  if (
    await FlowService.existsFlowByPath({
      workspace: workspace,
      path: remotePath,
    })
  ) {
    console.log(colors.bold.yellow("Updating existing flow..."));
    await FlowService.updateFlow({
      workspace: workspace,
      path: remotePath,
      requestBody: {
        path: remotePath,
        summary: data.summary,
        value: data.value,
        schema: data.schema,
        description: data.description,
      },
    });
  } else {
    console.log(colors.bold.yellow("Creating new flow..."));
    await FlowService.createFlow({
      workspace: workspace,
      requestBody: {
        path: remotePath,
        summary: data.summary,
        value: data.value,
        schema: data.schema,
        description: data.description,
      },
    });
  }
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
    .header(["path", "summary", "edited at", "edited by"])
    .padding(2)
    .border(true)
    .body(
      total.map((x) => [
        x.path,
        x.summary,
        x.edited_at,
        x.edited_by,
        x.description ?? "-",
      ]),
    )
    .render();
}
async function run(
  opts: GlobalOptions & {
    input: string[];
    silent: boolean;
  },
  path: string,
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const input = await resolve(opts.input);

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
    "push a local flow spec. This overrides any remote versions.",
  )
  .arguments("<file_path:string> <remote_path:string>")
  .action(push as any)
  .command("run", "run a flow by path.")
  .arguments("<path:string>")
  .option(
    "-i --input [inputs...:string]",
    "Inputs specified as JSON objects or simply as <name>=<value>. Supports file inputs using @<filename> and stdin using @- these also need to be formatted as JSON. Later inputs override earlier ones.",
  )
  .option(
    "-s --silent",
    "Do not ouput anything other then the final output. Useful for scripting.",
  )
  .action(run as any);

export default command;
