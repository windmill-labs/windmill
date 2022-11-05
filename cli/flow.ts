import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/command.ts";
import { FlowService } from "https://deno.land/x/windmill@v1.41.0/mod.ts";
import { GlobalOptions } from "./types.ts";
import {
  Flow,
  OpenFlow,
} from "https://deno.land/x/windmill@v1.41.0/windmill-api/index.ts";
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import { getContext } from "./context.ts";
import { Table } from "https://deno.land/x/cliffy@v0.25.4/table/table.ts";

type Options = GlobalOptions;

async function push(opts: Options, filePath: string, remotePath: string) {
  const { workspace } = await getContext(opts);

  if (!(remotePath.startsWith("g") || remotePath.startsWith("u"))) {
    console.log(
      colors.red(
        "Given remote path looks invalid. Remote paths are typicall of the form <u|g>/<username|group>/..."
      )
    );
    return;
  }
  await pushFlow(filePath, workspace, remotePath);
  console.log(colors.bold.underline.green("Flow successfully pushed"));
}

export async function pushFlow(
  filePath: string,
  workspace: string,
  remotePath: string
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
  const { workspace } = await getContext(opts);

  let page = 0;
  const perPage = 10;
  const total: Flow[] = [];
  while (true) {
    const res = await FlowService.listFlows({
      workspace,
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
      ])
    )
    .render();
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
  .action(push as any);

export default command;
