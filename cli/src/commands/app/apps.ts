// deno-lint-ignore-file no-explicit-any
import { requireLogin } from "../../../src/core/auth.ts";
import { resolveWorkspace, validatePath } from "../../../src/core/context.ts";
import { colors, Command, log, SEP, Table, yamlParseFile } from "../../../deps.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { ListableApp, Policy } from "../../../gen/types.gen.ts";

import { GlobalOptions, isSuperset } from "../../types.ts";
import { readInlinePathSync } from "../../../src/utils/utils.ts";

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
  remotePath = remotePath.replaceAll(SEP, "/");
  let app: any = undefined;
  // deleting old app if it exists in raw mode
  try {
    app = await wmill.getAppByPath({
      workspace,
      path: remotePath,
    });
  } catch {
    //ignore
  }

  if (!localPath.endsWith(SEP)) {
    localPath += SEP;
  }
  const path = localPath + "app.yaml";
  const localApp = (await yamlParseFile(path)) as AppFile;

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
            o["content"] = readInlinePathSync(basePath);
          }
          if (o["lock"] && o["lock"].startsWith("!inline")) {
            const basePath = localPath + o["lock"].split(" ")[1];
            o["lock"] = readInlinePathSync(basePath);
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
    await wmill.updateApp({
      workspace,
      path: remotePath,
      requestBody: {
        deployment_message: message,
        ...localApp,
      },
    });
  } else {
    log.info(colors.yellow.bold("Creating new app..."));

    await wmill.createApp({
      workspace,
      requestBody: {
        path: remotePath,
        deployment_message: message,
        ...localApp,
      },
    });
  }
}

async function list(opts: GlobalOptions & { includeDraftOnly?: boolean }) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  let page = 0;
  const perPage = 10;
  const total: ListableApp[] = [];
  while (true) {
    const res = await wmill.listApps({
      workspace: workspace.workspaceId,
      page,
      perPage,
      includeDraftOnly: opts.includeDraftOnly ?? false,
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
