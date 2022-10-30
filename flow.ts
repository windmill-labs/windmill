import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/command.ts";
import {
  FlowService,
  setClient,
} from "https://deno.land/x/windmill@v1.41.0/mod.ts";
import { GlobalOptions } from "./types.ts";
import { OpenFlowWPath } from "https://deno.land/x/windmill@v1.41.0/windmill-api/index.ts";
import { getToken } from "./login.ts";
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";

type Options = GlobalOptions;

async function push({ baseUrl }: Options, filePath: string) {
  setClient(await getToken(baseUrl), baseUrl);
  const data: OpenFlowWPath & { workspace_id: string } = JSON.parse(
    await Deno.readTextFile(filePath)
  );
  if (
    await FlowService.existsFlowByPath({
      workspace: data.workspace_id,
      path: data.path,
    })
  ) {
    console.log(colors.bold.yellow("Updating existing flow..."));
    await FlowService.updateFlow({
      workspace: data.workspace_id,
      path: data.path,
      requestBody: {
        path: data.path,
        summary: data.summary,
        value: data.value,
        schema: data.schema,
        description: data.description,
      },
    });
  } else {
    console.log(colors.bold.yellow("Creating new flow..."));
    await FlowService.createFlow({
      workspace: data.workspace_id,
      requestBody: {
        path: data.path,
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
  .arguments("<file_path:string>")
  .action(push as any);

export default command;
