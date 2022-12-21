// deno-lint-ignore-file no-explicit-any
import { colors } from "https://deno.land/x/cliffy@v0.25.4/ansi/colors.ts";
import { Command } from "https://deno.land/x/cliffy@v0.25.4/command/command.ts";
import { Table } from "https://deno.land/x/cliffy@v0.25.4/table/table.ts";
import { VariableService } from "https://deno.land/x/windmill@v1.50.0/mod.ts";
import { requireLogin, resolveWorkspace, validatePath } from "./context.ts";
import { GlobalOptions } from "./types.ts";

async function list(opts: GlobalOptions) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const variables = await VariableService.listVariable({
    workspace: workspace.workspaceId,
  });

  new Table()
    .header(["Path", "Is Secret", "Account", "Value"])
    .padding(2)
    .border(true)
    .body(
      variables.map((x) => [
        x.path,
        x.is_secret ? "true" : "false",
        x.account ?? "-",
        x.value ?? "-",
      ]),
    )
    .render();
}

type VariableFile = {
  value: string;
  is_secret: boolean;
  description: string;
  account?: number;
  is_oauth?: boolean;
};

async function push(opts: GlobalOptions, filePath: string, remotePath: string) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (!await validatePath(opts, remotePath)) {
    return;
  }

  const fstat = await Deno.stat(filePath);
  if (!fstat.isFile) {
    throw new Error("file path must refer to a file.");
  }

  console.log(colors.bold.yellow("Pushing variable..."));

  await pushVariable(workspace.workspaceId, filePath, remotePath);
  console.log(colors.bold.underline.green("Variable successfully pushed"));
}

export async function pushVariable(
  workspace: string,
  filePath: string,
  remotePath: string,
) {
  const data: VariableFile = JSON.parse(await Deno.readTextFile(filePath));
  if (await VariableService.existsVariable({ workspace, path: remotePath })) {
    const existing = await VariableService.getVariable({
      workspace: workspace,
      path: remotePath,
    });
    if (existing.is_oauth != data.is_oauth) {
      console.log(
        colors.red.underline.bold(
          "Remote variable at " +
            remotePath +
            " exists & has a different oauth state. This cannot be updated. If you wish to do this anyways, consider deleting the remote resource.",
        ),
      );
      return;
    }

    if (existing.account != data.account) {
      console.log(
        colors.red.underline.bold(
          "Remote variable at " +
            remotePath +
            " exists & has a different account state. This cannot be updated. If you wish to do this anyways, consider deleting the remote resource.",
        ),
      );
      return;
    }

    if (existing.is_secret && !data.is_secret) {
      console.log(
        colors.red.underline.bold(
          "Remote variable at " +
            remotePath +
            " exists & is secret. Variables cannot be updated to be no longer secret. If you wish to do this anyways, consider deleting the remote resource.",
        ),
      );
      return;
    }

    const actual_secret = data.is_secret ? true : undefined;

    console.log(colors.yellow("Updating existing variable..."));
    await VariableService.updateVariable({
      workspace,
      path: remotePath,
      requestBody: {
        description: data.description,
        is_secret: actual_secret,
        path: remotePath,
        value: data.value,
      },
    });
  } else {
    console.log(colors.yellow("Creating new variable..."));
    await VariableService.createVariable({
      workspace,
      requestBody: {
        path: remotePath,
        description: data.description,
        is_secret: data.is_secret,
        value: data.value,
        account: data.account,
        is_oauth: data.is_oauth,
      },
    });
  }
}

const command = new Command()
  .description("variable related commands")
  .action(list as any)
  .command(
    "push",
    "Push a local variable spec. This overrides any remote versions.",
  )
  .arguments("<file_path:string> <remote_path:string>")
  .action(push as any);

export default command;
