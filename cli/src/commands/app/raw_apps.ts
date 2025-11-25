// deno-lint-ignore-file no-explicit-any
import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace, validatePath } from "../../core/context.ts";
import {
  colors,
  log,
  SEP,
  windmillUtils,
  yamlParseFile,
} from "../../../deps.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { Policy } from "../../../gen/types.gen.ts";

import { GlobalOptions, isSuperset } from "../../types.ts";

import { replaceInlineScripts } from "./apps.ts";
import { createBundle } from "./bundle.ts";

export interface AppFile {
  runnables: any;
  custom_path: string;
  public?: boolean;
  summary: string;
  policy: Policy;
}

const alreadySynced: string[] = [];

async function collectAppFiles(
  localPath: string
): Promise<Record<string, string>> {
  const files: Record<string, string> = {};

  async function readDirRecursive(dir: string, basePath: string = "") {
    for await (const entry of Deno.readDir(dir)) {
      const fullPath = dir + entry.name;
      const relativePath = basePath + entry.name;

      if (entry.isDirectory) {
        // Skip the runnables subfolder
        if (entry.name === "runnables") {
          continue;
        }
        await readDirRecursive(fullPath + SEP, relativePath + SEP);
      } else if (entry.isFile) {
        // Skip raw_app.yaml as it's metadata, not an app file
        if (relativePath === "raw_app.yaml") {
          continue;
        }
        const content = await Deno.readTextFile(fullPath);
        files[relativePath] = content;
      }
    }
  }

  await readDirRecursive(localPath);
  return files;
}

export async function pushRawApp(
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
  if (app?.["policy"]?.["execution_mode"] == "anonymous") {
    app.public = true;
  }
  // console.log(app);
  if (app) {
    app.policy = undefined;
  }

  if (!localPath.endsWith(SEP)) {
    localPath += SEP;
  }
  const path = localPath + "raw_app.yaml";
  const localApp = (await yamlParseFile(path)) as AppFile;
  replaceInlineScripts(localApp.runnables, localPath);
  await generatingPolicy(localApp, remotePath, localApp?.["public"] ?? false);
  const files = await collectAppFiles(localPath);
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
        value: { runnables: localApp.runnables, files },
        summary: localApp.summary,
        policy: localApp.policy,
      },
    });
  } else {
    log.info(colors.yellow.bold("Creating new app..."));
    const { js, css } = await createBundle({
      entryPoint: localPath + "index.tsx",
      production: true,
      minify: true,
    });
    await wmill.createAppRaw({
      workspace,
      formData: {
        app: {
          value: { runnables: localApp.runnables, files },
          path: remotePath,
          summary: localApp.summary,
          policy: localApp.policy,
          deployment_message: message,
          custom_path: localApp.custom_path,
        },
        js,
        css,
      },
    });
    // await wmill.createApp({
    //   workspace,
    //   requestBody: {
    //     path: remotePath,
    //     deployment_message: message,
    //     value: { runnables: localApp.runnables, files },
    //     summary: localApp.summary,
    //     policy: localApp.policy,
    //   },
    // });
  }
}

export async function generatingPolicy(
  app: any,
  path: string,
  publicApp: boolean
) {
  log.info(colors.gray(`Generating fresh policy for app ${path}...`));
  try {
    app.policy = await windmillUtils.updateRawAppPolicy(
      app.runnables,
      app.policy
    );
    app.policy.execution_mode = publicApp ? "anonymous" : "publisher";
  } catch (e) {
    log.error(colors.red(`Error generating policy for app ${path}: ${e}`));
    throw e;
  }
}

async function pushRawAppCommand(
  opts: GlobalOptions,
  filePath: string,
  remotePath: string
) {
  if (!validatePath(remotePath)) {
    return;
  }
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  await pushRawApp(workspace.workspaceId, remotePath, filePath);
  log.info(colors.bold.underline.green("Raw app pushed"));
}
