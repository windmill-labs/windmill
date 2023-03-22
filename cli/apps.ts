import { requireLogin, resolveWorkspace, validatePath } from "./context.ts";
import { Any, decoverto, model, property } from "./decoverto.ts";
import {
  AppService,
  AppWithLastVersion,
  colors,
  Command,
  ListableApp,
  microdiff,
  Policy,
  Table,
} from "./deps.ts";
import {
  Difference,
  GlobalOptions,
  PushDiffs,
  Resource,
  setValueByPath,
} from "./types.ts";

@model()
export class AppFile implements Resource, PushDiffs {
  @property(Any)
  value: any;
  @property(() => String)
  summary: string;
  @property(Any)
  policy: Policy;

  constructor(value: string, summary: string, policy: Policy) {
    this.value = value;
    this.summary = summary;
    this.policy = policy;
  }
  async pushDiffs(
    workspace: string,
    remotePath: string,
    diffs: Difference[]
  ): Promise<void> {
    let app: AppWithLastVersion | undefined = undefined;
    try {
      app = await AppService.getAppByPath({ workspace, path: remotePath });
    } catch (e) {}

    if (app) {
      console.log(
        colors.bold.yellow(
          `Applying ${diffs.length} diffs to existing app... ${remotePath}`
        )
      );
      const changeset: {
        summary?: string | undefined;
        value?: any;
        policy?: Policy | undefined;
      } = {};
      for (const diff of diffs) {
        if (
          diff.type !== "REMOVE" &&
          diff.path[0] !== "value" &&
          diff.path[0] !== "policy" &&
          (diff.path.length !== 1 ||
            !["summary"].includes(diff.path[0] as string))
        ) {
          throw new Error("Invalid app diff with path " + diff.path);
        }
        if (diff.type === "CREATE" || diff.type === "CHANGE") {
          setValueByPath(changeset, diff.path, diff.value);
        } else if (diff.type === "REMOVE") {
          setValueByPath(changeset, diff.path, null);
        }
      }

      if (
        (!changeset?.policy ||
          JSON.stringify(changeset?.policy) == JSON.stringify(app.policy)) &&
        (!changeset?.value ||
          JSON.stringify(changeset?.value) == JSON.stringify(app.value)) &&
        (!changeset?.summary || changeset.summary == app.summary)
      ) {
        console.log(
          colors.yellow(`No changes to push for app ${remotePath}, skipping`)
        );
        return;
      }

      const hasChanges = Object.values(changeset).some(
        (v) => v !== null && typeof v !== "undefined"
      );
      if (!hasChanges) {
        return;
      }

      await AppService.updateApp({
        workspace,
        path: remotePath,
        requestBody: changeset,
      });
    } else {
      console.log(colors.yellow.bold("Creating new app..."));

      await AppService.createApp({
        workspace,
        requestBody: {
          path: remotePath,
          policy: this.policy,
          summary: this.summary,
          value: this.value,
        },
      });
    }
  }
  async push(workspace: string, remotePath: string): Promise<void> {
    await this.pushDiffs(
      workspace,
      remotePath,
      microdiff({}, this, { cyclesFix: false })
    );
  }
}

async function list(opts: GlobalOptions) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  let page = 0;
  const perPage = 10;
  const total: ListableApp[] = [];
  while (true) {
    const res = await AppService.listApps({
      workspace: workspace.workspaceId,
      page,
      perPage,
    });
    page += 1;
    total.push(...res);
    if (res.length < perPage) {
      break;
    }
  }

  new Table()
    .header(["path", "summary"])
    .padding(2)
    .border(true)
    .body(total.map((x) => [x.path, x.summary]))
    .render();
}

async function push(opts: GlobalOptions, filePath: string) {
  const remotePath = filePath.split(".")[0];
  if (!validatePath(remotePath)) {
    return;
  }
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  await pushApp(filePath, workspace.workspaceId, remotePath);
  console.log(colors.bold.underline.green("App pushed"));
}

export async function pushApp(
  filePath: string,
  workspace: string,
  remotePath: string
) {
  const data = decoverto
    .type(AppFile)
    .rawToInstance(await Deno.readTextFile(filePath));
  await data.push(workspace, remotePath);
}

const command = new Command()
  .description("app related commands")
  .action(list as any)
  .command("push", "push a local app ")
  .arguments("<file_path:file>")
  .action(push as any);

export default command;
