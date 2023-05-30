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
  Confirm,
  ListableVariable,
  log,
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
  plainSecrets: boolean,
  raw: boolean
): Promise<void> {
  remotePath = removeType(remotePath, "variable");
  log.debug(`Processing local variable ${remotePath}`);

  if (raw) {
    try {
      variable = await VariableService.getVariable({
        workspace: workspace,
        path: remotePath,
        decryptSecret: plainSecrets,
      });
      log.debug(`Variable ${remotePath} exists on remote`);
    } catch {
      log.debug(`Variable ${remotePath} does not exist on remote`);
    }
  }

  if (variable) {
    if (isSuperset(localVariable, variable)) {
      log.debug(`Variable ${remotePath} is up-to-date`);
      return;
    }
    log.debug(`Variable ${remotePath} is not up-to-date, updating`);

    await VariableService.updateVariable({
      workspace,
      path: remotePath,
      alreadyEncrypted: !plainSecrets,
      requestBody: {
        ...localVariable,
        is_secret:
          localVariable.is_secret && !variable.is_secret ? true : undefined,
      },
    });
  } else {
    log.info(colors.yellow.bold(`Creating new variable ${remotePath}...`));
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

  log.info(colors.bold.yellow("Pushing variable..."));

  await pushVariable(
    workspace.workspaceId,
    remotePath,
    undefined,
    parseFromFile(filePath),
    opts.plainSecrets,
    true
  );
  log.info(colors.bold.underline.green(`Variable ${remotePath} pushed`));
}

async function add(
  opts: GlobalOptions & { public?: boolean },
  value: string,
  remotePath: string
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (!validatePath(remotePath)) {
    return;
  }

  if (
    await VariableService.existsVariable({
      workspace: workspace.workspaceId,
      path: remotePath,
    })
  ) {
    if (
      !(await Confirm.prompt({
        message: `Variable already exist, do you want to update its value?`,
        default: true,
      }))
    ) {
      return;
    }
    log.info(colors.bold.yellow("Updating variable..."));
  }

  log.info(colors.bold.yellow("Pushing variable..."));

  await pushVariable(
    workspace.workspaceId,
    remotePath + ".variable.yaml",
    undefined,
    {
      value,
      is_secret: !opts.public,
      description: "",
    },
    true,
    true
  );
  log.info(colors.bold.underline.green(`Variable ${remotePath} pushed`));
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
  .action(push as any)
  .command(
    "add",
    "Create a new variable on the remote. This will update the variable if it already exists."
  )
  .arguments("<value:string> <remote_path:string>")
  .option("--public", "Make a public variable")
  .action(add as any);

export default command;
