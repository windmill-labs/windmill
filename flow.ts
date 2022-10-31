import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/command.ts";
import {
  FlowService,
  setClient,
} from "https://deno.land/x/windmill@v1.41.0/mod.ts";
import { GlobalOptions } from "./types.ts";
import { OpenFlow } from "https://deno.land/x/windmill@v1.41.0/windmill-api/index.ts";
import { getToken } from "./login.ts";
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import { getDefaultWorkspaceId } from "./workspace.ts";

type Options = GlobalOptions;

async function push(
  { baseUrl, workspace }: Options,
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
  const data: OpenFlow = JSON.parse(await Deno.readTextFile(filePath));
  if (
    await FlowService.existsFlowByPath({
      workspace: workspaceId,
      path: remotePath,
    })
  ) {
    console.log(colors.bold.yellow("Updating existing flow..."));
    await FlowService.updateFlow({
      workspace: workspaceId,
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
      workspace: workspaceId,
      requestBody: {
        path: remotePath,
        summary: data.summary,
        value: data.value,
        schema: data.schema,
        description: data.description,
      },
    });
  }
  console.log(colors.bold.underline.green("Flow successfully pushed"));
}

const command = new Command()
  .description("flow related commands")
  .command(
    "push",
    "push a local flow spec. This overrides any remote versions."
  )
  .arguments("<file_path:string> <remote_path:string>")
  .action(push as any);

export default command;
