import { requireLogin, resolveWorkspace, validatePath } from "./context.ts";
import {
  AppService,
  AppWithLastVersion,
  colors,
  Command,
  ListableApp,
  Policy,
  Table,
} from "./deps.ts";
import { GlobalOptions } from "./types.ts";

export interface App {
  value: any;
  summary: string;
  policy: Policy;
}

export async function pushApp(
  workspace: string,
  remotePath: string,
  localApp: App
): Promise<void> {
  let app: AppWithLastVersion | undefined = undefined;
  try {
    app = await AppService.getAppByPath({ workspace, path: remotePath });
  } catch (e) {}

  if (app) {
    // console.log(
    //   colors.bold.yellow(
    //     `Applying ${diffs.length} diffs to existing app... ${remotePath}`
    //   )
    // );

    await AppService.updateApp({
      workspace,
      path: remotePath,
      requestBody: {
        path: remotePath,
        ...localApp,
      },
    });
  } else {
    console.log(colors.yellow.bold("Creating new app..."));

    await AppService.createApp({
      workspace,
      requestBody: {
        path: remotePath,
        ...localApp,
      },
    });
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

  await pushApp(workspace.workspaceId, remotePath);
  console.log(colors.bold.underline.green("App pushed"));
}

const command = new Command()
  .description("app related commands")
  .action(list as any)
  .command("push", "push a local app ")
  .arguments("<file_path:file>")
  .action(push as any);

export default command;
