import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/command.ts";
import {
  ResourceService,
  setClient,
} from "https://deno.land/x/windmill@v1.41.0/mod.ts";
import { GlobalOptions } from "./types.ts";
import { getToken } from "./login.ts";
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import { getDefaultWorkspaceId } from "./workspace.ts";

type ResourceFile = {
  value: any;
  description?: string;
  resource_type: string;
  is_oauth?: boolean;
};

type PushOptions = GlobalOptions;
async function push(
  { baseUrl, workspace }: PushOptions,
  filePath: string,
  remotePath: string
) {
  setClient(await getToken(baseUrl), baseUrl);
  const workspaceId = workspace ?? (await getDefaultWorkspaceId(baseUrl));
  if (!workspaceId) {
    console.log(colors.red("No default workspace set and no override given."));
    return;
  }

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

  const data: ResourceFile = JSON.parse(await Deno.readTextFile(filePath));

  console.log(colors.bold.yellow("Pushing resource..."));

  // TODO:
  if (
    await ResourceService.existsResource({
      workspace: workspaceId,
      path: remotePath,
    })
  ) {
    console.log(colors.yellow("Updating existing resource..."));
    const existing = await ResourceService.getResource({
      workspace: workspaceId,
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
      workspace: workspaceId,
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
      workspace: workspaceId,
      requestBody: {
        path: remotePath,
        resource_type: data.resource_type,
        value: data.value,
        description: data.description,
        is_oauth: data.is_oauth,
      },
    });
  }
  console.log(colors.bold.underline.green("Resource successfully pushed"));
}

const command = new Command()
  .description("resource related commands")
  .command(
    "push",
    "push a local resource spec. This overrides any remote versions."
  )
  .arguments("<file_path:string> <remote_path:string>")
  .action(push as any);

export default command;
