import { stat, writeFile } from "node:fs/promises";
import { stringify as yamlStringify } from "yaml";

import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace, validatePath } from "../../core/context.ts";
import {
  GlobalOptions,
  isSuperset,
  parseFromFile,
  removeType,
} from "../../types.ts";
import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import { colors } from "@cliffy/ansi/colors";
import { Confirm } from "@cliffy/prompt/confirm";
import * as log from "../../core/log.ts";
import { sep as SEP } from "node:path";

import * as wmill from "../../../gen/services.gen.ts";
import { ListableVariable } from "../../../gen/types.gen.ts";

async function list(opts: GlobalOptions & { json?: boolean }) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  const variables = await wmill.listVariable({
    workspace: workspace.workspaceId,
  });

  if (opts.json) {
    console.log(JSON.stringify(variables));
  } else {
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
}

async function newVariable(opts: GlobalOptions, path: string) {
  if (!validatePath(path)) {
    return;
  }
  const filePath = path + ".variable.yaml";
  try {
    await stat(filePath);
    throw new Error("File already exists: " + filePath);
  } catch (e: any) {
    if (e.message?.startsWith("File already exists")) throw e;
  }
  const template: VariableFile = {
    value: "",
    is_secret: false,
    description: "",
  };
  await writeFile(filePath, yamlStringify(template as Record<string, any>), {
    flag: "wx",
    encoding: "utf-8",
  });
  log.info(colors.green(`Created ${filePath}`));
}

async function get(opts: GlobalOptions & { json?: boolean }, path: string) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const v = await wmill.getVariable({
    workspace: workspace.workspaceId,
    path,
  });
  if (opts.json) {
    console.log(JSON.stringify(v));
  } else {
    console.log(colors.bold("Path:") + " " + v.path);
    console.log(colors.bold("Value:") + " " + (v.value ?? "-"));
    console.log(colors.bold("Is Secret:") + " " + (v.is_secret ? "true" : "false"));
    console.log(colors.bold("Description:") + " " + (v.description ?? ""));
    console.log(colors.bold("Account:") + " " + (v.account ?? "-"));
  }
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
  log.debug(`Processing local variable ${remotePath}`);

  try {
    variable = await wmill.getVariable({
      workspace: workspace,
      path: remotePath.replaceAll(SEP, "/"),
      decryptSecret: plainSecrets,
      includeEncrypted: true,
    });
    log.debug(`Variable ${remotePath} exists on remote`);
  } catch {
    log.debug(`Variable ${remotePath} does not exist on remote`);
  }

  if (variable) {
    if (isSuperset(localVariable, variable)) {
      log.debug(`Variable ${remotePath} is up-to-date`);
      return;
    }

    log.debug(`Variable ${remotePath} is not up-to-date, updating`);

    await wmill.updateVariable({
      workspace,
      path: remotePath.replaceAll(SEP, "/"),
      alreadyEncrypted: !plainSecrets,
      requestBody: {
        ...localVariable,
        is_secret:
          localVariable.is_secret && !variable.is_secret ? true : undefined,
      },
    });
  } else {
    log.info(colors.yellow.bold(`Creating new variable ${remotePath}...`));
    await wmill.createVariable({
      workspace,
      alreadyEncrypted: !plainSecrets,
      requestBody: {
        path: remotePath.replaceAll(SEP, "/"),
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

  const fstat = await stat(filePath);
  if (!fstat.isFile()) {
    throw new Error("file path must refer to a file.");
  }

  log.info(colors.bold.yellow("Pushing variable..."));

  await pushVariable(
    workspace.workspaceId,
    remotePath,
    undefined,
    parseFromFile(filePath),
    opts.plainSecrets ?? false
  );
  log.info(colors.bold.underline.green(`Variable ${remotePath} pushed`));
}

async function add(
  opts: GlobalOptions & { public?: boolean; plainSecrets?: boolean },
  value: string,
  remotePath: string
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  if (!validatePath(remotePath)) {
    return;
  }

  if (
    await wmill.existsVariable({
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
      is_secret: !opts.public && !opts.plainSecrets,
      description: "",
    },
    opts.plainSecrets ?? false
  );
  log.info(colors.bold.underline.green(`Variable ${remotePath} pushed`));
}

const command = new Command()
  .description("variable related commands")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("list", "list all variables")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("get", "get a variable's details")
  .arguments("<path:string>")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(get as any)
  .command("new", "create a new variable locally")
  .arguments("<path:string>")
  .action(newVariable as any)
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
  .option("--plain-secrets", "Push secrets as plain text")
  .option("--public", "Legacy option, use --plain-secrets instead")

  .action(add as any);

export default command;
