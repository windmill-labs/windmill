// deno-lint-ignore-file no-explicit-any
import { requireLogin, resolveWorkspace, validatePath } from "./context.ts";
import {
  AppService,
  colors,
  Command,
  ListableApp,
  log,
  Policy,
  SEP,
  Table,
  yamlParse,
} from "./deps.ts";
import { GlobalOptions, isSuperset } from "./types.ts";

export interface AppFile {
  value: any;
  summary: string;
  policy: Policy;
}

const alreadySynced: string[] = [];

export async function pushApp(
  workspace: string,
  remotePath: string,
  localPath: string,
  message?: string
): Promise<void> {
  if (alreadySynced.includes(localPath)) {
    return;
  }
  alreadySynced.push(localPath);

  let app: any = undefined;
  // deleting old app if it exists in raw mode
  try {
    app = await AppService.getAppByPath({
      workspace,
      path: remotePath.replaceAll(SEP, "/"),
    });
  } catch {
    //ignore
  }

  if (!localPath.endsWith(SEP)) {
    localPath += SEP;
  }
  const localAppRaw = await Deno.readTextFile(localPath + "app.yaml");
  const localApp = yamlParse(localAppRaw) as AppFile;

  function replaceInlineScripts(rec: any) {
    if (!rec) {
      return;
    }
    if (typeof rec == "object") {
      return Object.entries(rec).flatMap(([k, v]) => {
        if (k == "inlineScript" && typeof v == "object") {
          const o: Record<string, any> = v as any;
          if (o["content"] && o["content"].startsWith("!inline")) {
            const basePath = localPath + o["content"].split(" ")[1];
            o["content"] = Deno.readTextFileSync(basePath);
          }
          if (o["lock"] && o["lock"].startsWith("!inline")) {
            const basePath = localPath + o["lock"].split(" ")[1];
            o["lock"] = Deno.readTextFileSync(basePath);
          }
        } else {
          replaceInlineScripts(v);
        }
      });
    }
    return [];
  }

  replaceInlineScripts(localApp.value);

  if (app) {
    if (isSuperset(localApp, app)) {
      log.info(colors.green(`App ${remotePath} is up to date`));
      return;
    }
    log.info(colors.bold.yellow(`Updating app ${remotePath}...`));
    await AppService.updateApp({
      workspace,
      path: remotePath.replaceAll(SEP, "/"),
      requestBody: {
        deployment_message: message,
        ...localApp,
      },
    });
  } else {
    log.info(colors.yellow.bold("Creating new app..."));

    await AppService.createApp({
      workspace,
      requestBody: {
        path: remotePath.replaceAll(SEP, "/"),
        deployment_message: message,
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

async function push(opts: GlobalOptions, filePath: string, remotePath: string) {
  if (!validatePath(remotePath)) {
    return;
  }
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  await pushApp(workspace.workspaceId, remotePath, filePath);
  log.info(colors.bold.underline.green("Flow pushed"));
}

const command = new Command()
  .description("app related commands")
  .action(list as any)
  .command("push", "push a local app ")
  .arguments("<file_path:string> <remote_path:string>")
  .action(push as any);

export default command;
