// deno-lint-ignore-file no-explicit-any
import { requireLogin, resolveWorkspace, validatePath } from "./context.ts";
import {
  Difference,
  GlobalOptions,
  PushDiffs,
  Resource,
  setValueByPath,
} from "./types.ts";
import {
  colors,
  Command,
  EditVariable,
  microdiff,
  Table,
  VariableService,
} from "./deps.ts";
import { decoverto, model, property } from "./decoverto.ts";

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

@model()
export class VariableFile implements Resource, PushDiffs {
  @property(() => String)
  value: string;
  @property(() => Boolean)
  is_secret: boolean;
  @property(() => String)
  description: string;
  @property(() => Number)
  account?: number;
  @property(() => Boolean)
  is_oauth?: boolean;

  constructor(value: string, is_secret: boolean, description: string) {
    this.value = value;
    this.is_secret = is_secret;
    this.description = description;
  }
  async pushDiffs(
    workspace: string,
    remotePath: string,
    diffs: Difference[],
  ): Promise<void> {
    if (await VariableService.existsVariable({ workspace, path: remotePath })) {
      console.log(
        colors.bold.yellow(
          `Applying ${diffs.length} diffs to existing variable... ${remotePath}`,
        ),
      );
      const changeset: EditVariable = {};
      for (const diff of diffs) {
        if (
          diff.type !== "REMOVE" &&
          (
            diff.path.length !== 1 ||
            !["path", "value", "is_secret", "description", "account", "is_oauth"].includes(
              diff.path[0] as string,
            )
          )
        ) {
          console.log(colors.red("Invalid variable diff with path " + diff.path));
          throw new Error("Invalid variable diff with path " + diff.path);
        }
        if (diff.type === "CREATE" || diff.type === "CHANGE") {
          setValueByPath(changeset, diff.path, diff.value);
        } else if (diff.type === "REMOVE") {
          setValueByPath(changeset, diff.path, null);
        }
      }

      const hasChanges = Object.values(changeset).some((v) =>
        v !== null && typeof v !== "undefined"
      );
      if (!hasChanges) {
        return;
      }

      await VariableService.updateVariable({
        workspace,
        path: remotePath,
        alreadyEncrypted: true,
        requestBody: changeset,
      });

    } else {
      console.log(colors.yellow.bold("Creating new variable..."));
      await VariableService.createVariable({
        workspace,
        alreadyEncrypted: true,
        requestBody: {
          path: remotePath,
          description: this.description,
          is_secret: this.is_secret,
          value: this.value,
          account: this.account,
          is_oauth: this.is_oauth,
        },
      });
    }
  }
  async push(workspace: string, remotePath: string): Promise<void> {
    await this.pushDiffs(
      workspace,
      remotePath,
      microdiff({}, this, { cyclesFix: false }),
    );
  }
}

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
  console.log(colors.bold.underline.green(`Variable ${remotePath} pushed`));
}

export async function pushVariable(
  workspace: string,
  filePath: string,
  remotePath: string,
) {
  const data = decoverto.type(VariableFile).rawToInstance(
    await Deno.readTextFile(filePath),
  );
  await data.push(workspace, remotePath);
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
