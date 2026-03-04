import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace, validatePath } from "../../core/context.ts";
import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { sep as SEP } from "node:path";
import * as windmillUtils from "@windmill-labs/shared-utils";
import { yamlParseFile } from "../../utils/yaml.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { ListableApp, Policy } from "../../../gen/types.gen.ts";

import { GlobalOptions, isSuperset } from "../../types.ts";
import { readInlinePathSync } from "../../utils/utils.ts";
import devCommand from "./dev.ts";
import lintCommand from "./lint.ts";
import newCommand from "./new.ts";
import generateAgentsCommand from "./generate_agents.ts";
import { isVersionsGeq1585 } from "../sync/global.ts";
import type { PermissionedAsContext } from "../../core/permissioned_as.ts";
import { resolvePermissionedAsEmail, lookupUsernameByEmail } from "../../core/permissioned_as.ts";

export interface AppFile {
  value: any;
  public?: boolean;
  summary: string;
  policy: Policy;
}

const alreadySynced: string[] = [];

function respecializeFields(fields: Record<string, any>) {
  Object.entries(fields).forEach(([k, v]) => {
    if (typeof v == "object") {
      if (v.value !== undefined) {
        fields[k] = { value: v.value, type: "static" };
      } else if (v.ctx !== undefined) {
        fields[k] = { ctx: v.ctx, type: "ctx" };
      } else if (v.expr !== undefined) {
        fields[k] = {
          expr: v.expr,
          allowUserResources: v.allowUserResources,
          type: "javascript",
        };
      }
    }
  });
}

export function repopulateFields(runnables: Record<string, any>) {
  Object.values(runnables).forEach((v) => {
    if (typeof v == "object") {
      if (v.fields !== undefined) {
        respecializeFields(v.fields);
      }
    }
  });
}
export function replaceInlineScripts(
  rec: any,
  localPath: string,
  addType: boolean
) {
  if (!rec) {
    return;
  }
  if (typeof rec == "object") {
    return Object.entries(rec).flatMap(([k, v]) => {
      if (k == "runType") {
        if (addType) {
          if (isVersionsGeq1585()) {
            rec["type"] = "path";
          } else {
            rec["type"] = "runnableByPath";
          }
        }
      } else if (k == "inlineScript" && typeof v == "object") {
        if (addType) {
          if (isVersionsGeq1585()) {
            rec["type"] = "inline";
          } else {
            rec["type"] = "runnableByName";
          }
        }
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
        replaceInlineScripts(v, localPath, addType);
      }
    });
  }
  return [];
}
export function isExecutionModeAnonymous(app: any) {
  return app?.["policy"]?.["execution_mode"] == "anonymous";
}
export async function pushApp(
  workspace: string,
  remotePath: string,
  localPath: string,
  message?: string,
  permissionedAsContext?: PermissionedAsContext
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
  // Save remote policy ownership fields before clearing (needed for preserve)
  let remoteOnBehalfOf: string | undefined;
  let remoteOnBehalfOfEmail: string | undefined;
  if (app?.policy) {
    remoteOnBehalfOf = app.policy.on_behalf_of;
    remoteOnBehalfOfEmail = app.policy.on_behalf_of_email;
    log.debug(`Remote app ${remotePath} policy: on_behalf_of=${remoteOnBehalfOf}, on_behalf_of_email=${remoteOnBehalfOfEmail}`);
  }

  if (isExecutionModeAnonymous(app)) {
    app.public = true;
  }
  // console.log(app);
  if (app) {
    app.policy = undefined;
  }

  if (!localPath.endsWith(SEP)) {
    localPath += SEP;
  }
  const path = localPath + "app.yaml";
  const localApp = (await yamlParseFile(path)) as AppFile;

  replaceInlineScripts(localApp.value, localPath, true);
  await generatingPolicy(
    localApp,
    remotePath,
    localApp?.["public"] ??
      localApp?.["policy"]?.["execution_mode"] == "anonymous"
  );

  // Build preserve flags for permissioned_as
  const preserveFields: { preserve_on_behalf_of?: boolean } = {};
  if (permissionedAsContext?.userIsAdminOrDeployer) {
    if (app) {
      // Updating: inject remote's on_behalf_of into the freshly generated policy
      // The backend requires policy.on_behalf_of to be set for preserve to work
      if (localApp.policy && remoteOnBehalfOf) {
        (localApp.policy as any).on_behalf_of = remoteOnBehalfOf;
        (localApp.policy as any).on_behalf_of_email = remoteOnBehalfOfEmail;
        preserveFields.preserve_on_behalf_of = true;
      }
    } else {
      // Creating: apply defaultPermissionedAs rule if one matches
      const ruleEmail = resolvePermissionedAsEmail(
        remotePath,
        permissionedAsContext.rules
      );
      if (ruleEmail) {
        // Set both on_behalf_of and on_behalf_of_email on the policy
        // The backend requires on_behalf_of to be set for preserve to work
        if (localApp.policy) {
          const username = await lookupUsernameByEmail(
            workspace,
            ruleEmail,
            permissionedAsContext.emailToUsernameCache
          );
          (localApp.policy as any).on_behalf_of = `u/${username}`;
          (localApp.policy as any).on_behalf_of_email = ruleEmail;
        }
        preserveFields.preserve_on_behalf_of = true;
      }
    }
  }

  if (app) {
    if (isSuperset(localApp, app)) {
      log.info(colors.green(`App ${remotePath} is up to date`));
      return;
    }
    log.info(colors.bold.yellow(`Updating app ${remotePath}...`));
    const requestBody = {
      deployment_message: message,
      ...localApp,
      ...preserveFields,
    };
    log.debug(`App ${remotePath} update request: preserve_on_behalf_of=${requestBody.preserve_on_behalf_of}, policy.on_behalf_of=${(requestBody as any).policy?.on_behalf_of}, policy.on_behalf_of_email=${(requestBody as any).policy?.on_behalf_of_email}`);
    await wmill.updateApp({
      workspace,
      path: remotePath,
      requestBody,
    });
  } else {
    log.info(colors.yellow.bold("Creating new app..."));

    const requestBody = {
      path: remotePath,
      deployment_message: message,
      ...localApp,
      ...preserveFields,
    };
    log.debug(`App ${remotePath} create request: preserve_on_behalf_of=${requestBody.preserve_on_behalf_of}, policy.on_behalf_of=${(requestBody as any).policy?.on_behalf_of}, policy.on_behalf_of_email=${(requestBody as any).policy?.on_behalf_of_email}`);
    await wmill.createApp({
      workspace,
      requestBody,
    });
  }
}

export async function generatingPolicy(
  app: any,
  path: string,
  publicApp: boolean
) {
  log.info(colors.gray(`Generating fresh policy for app ${path}...`));
  try {
    app.policy = await windmillUtils.updatePolicy(app.value, undefined);
    app.policy.execution_mode = publicApp ? "anonymous" : "publisher";
  } catch (e) {
    log.error(colors.red(`Error generating policy for app ${path}: ${e}`));
    throw e;
  }
}

async function list(opts: GlobalOptions & { includeDraftOnly?: boolean; json?: boolean }) {
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

  if (opts.json) {
    console.log(JSON.stringify(total));
  } else {
    new Table()
      .header(["path", "summary"])
      .padding(2)
      .border(true)
      .body(total.map((x) => [x.path, x.summary]))
      .render();
  }
}

async function get(opts: GlobalOptions & { json?: boolean }, path: string) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);
  const a = await wmill.getAppByPath({
    workspace: workspace.workspaceId,
    path,
  });
  if (opts.json) {
    console.log(JSON.stringify(a));
  } else {
    console.log(colors.bold("Path:") + " " + a.path);
    console.log(colors.bold("Summary:") + " " + (a.summary ?? ""));
    console.log(colors.bold("Created by:") + " " + (a.created_by ?? ""));
  }
}

async function push(opts: GlobalOptions, filePath: string, remotePath: string) {
  if (!validatePath(remotePath)) {
    return;
  }
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  await pushApp(workspace.workspaceId, remotePath, filePath);
  log.info(colors.bold.underline.green("App pushed"));
}

async function setPermissionedAs(
  opts: GlobalOptions,
  appPath: string,
  email: string
) {
  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  // Look up username for the email — backend requires on_behalf_of (u/<username>) to be set
  const cache = new Map<string, string>();
  const username = await lookupUsernameByEmail(workspace.workspaceId, email, cache);

  await wmill.updateApp({
    workspace: workspace.workspaceId,
    path: appPath,
    requestBody: {
      policy: {
        on_behalf_of: `u/${username}`,
        on_behalf_of_email: email,
      } as any,
      preserve_on_behalf_of: true,
    },
  });
  log.info(
    colors.green(
      `Updated permissioned_as for app ${appPath} to ${email}`
    )
  );
}

const command = new Command()
  .description("app related commands")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("list", "list all apps")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(list as any)
  .command("get", "get an app's details")
  .arguments("<path:string>")
  .option("--json", "Output as JSON (for piping to jq)")
  .action(get as any)
  .command("push", "push a local app ")
  .arguments("<file_path:string> <remote_path:string>")
  .action(push as any)
  .command("dev", devCommand)
  .command("lint", lintCommand)
  .command("new", newCommand)
  .command("generate-agents", generateAgentsCommand)
  .command(
    "set-permissioned-as",
    "Set the on_behalf_of_email for an app (requires admin or wm_deployers group)"
  )
  .arguments("<path:string> <email:string>")
  .action(setPermissionedAs as any)
  .command(
    "generate-locks",
    "re-generate the lockfiles for app runnables inline scripts that have changed"
  )
  .arguments("[app_folder:string]")
  .option("--yes", "Skip confirmation prompt")
  .option("--dry-run", "Perform a dry run without making changes")
  .option(
    "--default-ts <runtime:string>",
    "Default TypeScript runtime (bun or deno)"
  )
  .action(async (opts: any, appFolder: string | undefined) => {
    const { generateLocksCommand } = await import("./app_metadata.ts");
    await generateLocksCommand(opts, appFolder);
  });

export default command;
