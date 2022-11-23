import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/command.ts";
import { ResourceService } from "https://deno.land/x/windmill@v1.50.0/mod.ts";
import { GlobalOptions } from "./types.ts";
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import { getContext } from "./context.ts";
import { Resource } from "https://deno.land/x/windmill@v1.50.0/windmill-api/index.ts";
import { Table } from "https://deno.land/x/cliffy@v0.25.4/table/table.ts";

type ResourceFile = {
  value: any;
  description?: string;
  resource_type: string;
  is_oauth?: boolean;
};

export async function pushResource(
  workspace: string,
  filePath: string,
  remotePath: string
) {
  const data: ResourceFile = JSON.parse(await Deno.readTextFile(filePath));
  if (
    await ResourceService.existsResource({
      workspace: workspace,
      path: remotePath,
    })
  ) {
    console.log(colors.yellow("Updating existing resource..."));
    const existing = await ResourceService.getResource({
      workspace: workspace,
      path: remotePath,
    });
    if (existing.resource_type != data.resource_type) {
      console.log(
        colors.red.underline.bold(
          "Remote resource at " +
            remotePath +
            " exists & has a different resource type. This cannot be updated. If you wish to do this anyways, consider deleting the remote resource."
        )
      );
      return;
    }
    if (existing.is_oauth != data.is_oauth) {
      console.log(
        colors.red.underline.bold(
          "Remote resource at " +
            remotePath +
            " exists & has a different oauth state. This cannot be updated. If you wish to do this anyways, consider deleting the remote resource."
        )
      );
      return;
    }
    await ResourceService.updateResource({
      workspace: workspace,
      path: remotePath,
      requestBody: {
        path: remotePath,
        value: data.value,
        description: data.description,
      },
    });
  } else {
    console.log(colors.yellow("Creating new resource..."));
    await ResourceService.createResource({
      workspace: workspace,
      requestBody: {
        path: remotePath,
        resource_type: data.resource_type,
        value: data.value,
        description: data.description,
        is_oauth: data.is_oauth,
      },
    });
  }
}

type PushOptions = GlobalOptions;
async function push(opts: PushOptions, filePath: string, remotePath: string) {
  const { workspace } = await getContext(opts);

  if (!(remotePath.startsWith("g") || remotePath.startsWith("u"))) {
    console.log(
      colors.red(
        "Given remote path looks invalid. Remote paths are typicall of the form <u|g>/<username|group>/..."
      )
    );
    return;
  }

  const fstat = await Deno.stat(filePath);
  if (!fstat.isFile) {
    throw new Error("file path must refer to a file.");
  }

  console.log(colors.bold.yellow("Pushing resource..."));

  await pushResource(workspace, filePath, remotePath);
  console.log(colors.bold.underline.green("Resource successfully pushed"));
}

async function list(opts: GlobalOptions) {
  const { workspace } = await getContext(opts);
  let page = 0;
  const perPage = 10;
  const total: Resource[] = [];
  while (true) {
    const res = await ResourceService.listResource({
      workspace,
      page,
      perPage,
    });
    total.push(...res);
    page += 1;
    if (res.length < perPage) {
      break;
    }
  }

  new Table()
    .header(["Path", "Resource Type"])
    .padding(2)
    .border(true)
    .body(total.map((x) => [x.path, x.resource_type]))
    .render();
}

const command = new Command()
  .description("resource related commands")
  .action(list as any)
  .command(
    "push",
    "push a local resource spec. This overrides any remote versions."
  )
  .arguments("<file_path:string> <remote_path:string>")
  .action(push as any);

export default command;
