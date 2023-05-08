// deno-lint-ignore-file no-explicit-any
import { requireLogin, resolveWorkspace, validatePath } from "./context.ts";
import {
  GlobalOptions,
  isSuperset,
  parseFromFile,
  removeType,
} from "./types.ts";
import {
  colors,
  Command,
  ListableVariable,
  Table,
  VariableService,
} from "./deps.ts";

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
      ])
    )
    .render();
}

export interface VariableFile {
  value: string;
  is_secret: boolean;
  description: string;
  account?: number;
  is_oauth?: boolean;
}

export async function pushVariable(
  workspace: string,
  remotePath: string,
  variable: VariableFile | ListableVariable | undefined,
  localVariable: VariableFile,
  plainSecrets: boolean
): Promise<void> {
  remotePath = removeType(remotePath, "variable");

  if (variable) {
    if (isSuperset(localVariable, variable)) {
      return;
    }

    await VariableService.updateVariable({
      workspace,
      path: remotePath,
      alreadyEncrypted: !plainSecrets,
      requestBody: {
        ...localVariable,
      },
    });
  } else {
    console.log(colors.yellow.bold(`Creating new variable ${remotePath}...`));
    await VariableService.createVariable({
      workspace,
      alreadyEncrypted: !plainSecrets,
      requestBody: {
        path: remotePath,
        ...localVariable,
      },
    });
  }
}

async function push(
  opts: GlobalOptions & { plainSecrets: boolean },
  filePath: string,
  remotePath: string
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (!validatePath(remotePath)) {
    return;
  }

  const fstat = await Deno.stat(filePath);
  if (!fstat.isFile) {
    throw new Error("file path must refer to a file.");
  }

  console.log(colors.bold.yellow("Pushing variable..."));

  let variable: ListableVariable | undefined = undefined;
  try {
    variable = await VariableService.getVariable({
      workspace: workspace.workspaceId,
      path: remotePath,
    });
  } catch {
    // resource type doesn't exist
  }

  await pushVariable(
    workspace.workspaceId,
    remotePath,
    variable,
    parseFromFile(filePath),
    opts.plainSecrets
  );
  console.log(colors.bold.underline.green(`Variable ${remotePath} pushed`));
}

const command = new Command()
  .description("variable related commands")
  .action(list as any)
  .command(
    "push",
    "Push a local variable spec. This overrides any remote versions."
  )
  .arguments("<file_path:string> <remote_path:string>")
  .option("--plain-secrets", "Push secrets as plain text")
  .action(push as any);

export default command;
