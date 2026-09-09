import { requireLogin } from "../../core/auth.ts";
import { resolveWorkspace, validatePath } from "../../core/context.ts";
import { Command } from "@cliffy/command";
import { Table } from "@cliffy/table";
import { colors } from "@cliffy/ansi/colors";
import * as log from "../../core/log.ts";
import { sep as SEP, isAbsolute, resolve as pathResolve, relative as pathRelative, basename } from "node:path";
import { stat } from "node:fs/promises";
import * as windmillUtils from "@windmill-labs/shared-utils";
import { yamlParseFile } from "../../utils/yaml.ts";
import * as wmill from "../../../gen/services.gen.ts";
import { ListableApp, Policy } from "../../../gen/types.gen.ts";

import { GlobalOptions, isSuperset } from "../../types.ts";
import {
  getWmillYamlPath,
  mergeConfigWithConfigFile,
  readEffectiveSyncBehavior,
} from "../../core/conf.ts";
import { readInlinePathSync } from "../../utils/utils.ts";
import devCommand from "./dev.ts";
import lintCommand from "./lint.ts";
import bundleCommand from "./bundle_command.ts";
import newCommand from "./new.ts";
import generateAgentsCommand from "./generate_agents.ts";
import { isVersionsGeq1585 } from "../sync/global.ts";
import type { PermissionedAsContext } from "../../core/permissioned_as.ts";
import { buildPermissionedAsContext } from "../../core/permissioned_as.ts";
import { applyExtraPermsDiff } from "../../core/extra_perms.ts";

export interface AppFile {
  guests?: boolean;
  value: any;
  public?: boolean;
  summary: string;
  policy: Policy;
  // Mirrors granular ACLs on the app path. Omitted from app.yaml when no
  // perms are set. The CLI applies diffs through /acls/add and /acls/remove
  // (see applyExtraPermsDiff) — never through update_app — so a perm-only
  // change never bumps the app version.
  extra_perms?: Record<string, boolean>;
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
export function isExecutionModeGuest(app: any) {
  return app?.["policy"]?.["execution_mode"] == "guest";
}
export type AppExecutionMode = "anonymous" | "guest" | "publisher" | "viewer";
/** The access mode is the one policy field a tracked app keeps, as `public` (anonymous)
 * or `guests` (guest); the rest of the policy is preserved from the deployed app on
 * push (see `generatingPolicy`). */
export function markAccessFromPolicy(app: any) {
  if (isExecutionModeAnonymous(app)) {
    app.public = true;
  } else if (isExecutionModeGuest(app)) {
    app.guests = true;
  }
}
/** The mode the tracked file states, or `undefined` when it states none — the
 * normal case, since a pull writes only the two open-access markers. `viewer`
 * and `publisher` have no marker of their own, so a file can only name them
 * through a policy block it was hand-written with. */
function statedExecutionMode(app: any): AppExecutionMode | undefined {
  if (app?.["public"] ?? isExecutionModeAnonymous(app)) {
    return "anonymous";
  }
  if (app?.["guests"] ?? isExecutionModeGuest(app)) {
    return "guest";
  }
  const mode = app?.["policy"]?.["execution_mode"];
  return mode === "viewer" || mode === "publisher" ? mode : undefined;
}

/** The mode this push deploys under. A file that states one is authoritative, in
 * both directions. Otherwise the two open-access markers are all it says, so
 * their absence closes a deployed `anonymous`/`guest` app back down to
 * `publisher` — while a deployed `viewer` is not a grant those markers revoke,
 * so it carries over rather than widening to `publisher`. */
export function executionModeForPush(
  localApp: any,
  deployedPolicy: Policy | undefined,
): AppExecutionMode {
  const stated = statedExecutionMode(localApp);
  if (stated) {
    return stated;
  }
  return deployedPolicy?.execution_mode === "viewer" ? "viewer" : "publisher";
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

  // `app.policy` is cleared a few lines down, so capture it first: it is the
  // base the regenerated policy is built on.
  const deployedPolicy: Policy | undefined = app?.policy;

  markAccessFromPolicy(app);
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
  // On create the backend applies folder defaults, so there is nothing to preserve.
  const preserveFields = preserveOnBehalfOfFields(
    remotePath,
    deployedPolicy,
    permissionedAsContext
  );
  await generatingPolicy(
    localApp,
    remotePath,
    executionModeForPush(localApp, deployedPolicy),
    basePolicy(localApp, deployedPolicy, !!preserveFields.preserve_on_behalf_of)
  );

  // extra_perms goes through /acls/* — strip from the body so a perms-only
  // edit never bumps the app version (see applyExtraPermsDiff for details).
  const { extra_perms: localPerms, ...localAppBody } = localApp as AppFile & {
    extra_perms?: Record<string, boolean>;
  };

  if (app) {
    if (isSuperset(localAppBody, app)) {
      log.info(colors.green(`App ${remotePath} is up to date`));
    } else {
      log.info(colors.bold.yellow(`Updating app ${remotePath}...`));
      await wmill.updateApp({
        workspace,
        path: remotePath,
        requestBody: {
          deployment_message: message,
          ...localAppBody,
          ...preserveFields,
          // Preserve any user draft at this path (see backend skip_draft_deletion).
          skip_draft_deletion: true,
        },
      });
    }
  } else {
    log.info(colors.yellow.bold("Creating new app..."));

    await wmill.createApp({
      workspace,
      requestBody: {
        path: remotePath,
        deployment_message: message,
        ...localAppBody,
        ...preserveFields,
        // Preserve any user draft at this path (see backend skip_draft_deletion).
        skip_draft_deletion: true,
      },
    });
  }

  // Independent perms sync via /acls/* — self-contained log + non-fatal errors.
  // No refetch: extra_perms is item-specific and folder perms are never merged
  // onto item.extra_perms, and the body sent to update_app / create_app omits
  // the field — so the value we already have from getAppByPath is authoritative.
  await applyExtraPermsDiff(
    workspace,
    "app",
    remotePath,
    localPerms,
    (app as any)?.extra_perms,
  );
}

export async function generatingPolicy(
  app: any,
  path: string,
  executionMode: AppExecutionMode,
  base: Policy | undefined
) {
  log.info(colors.gray(`Generating fresh policy for app ${path}...`));
  try {
    app.policy = await windmillUtils.updatePolicy(app.value, base);
    finalizeDerivedPolicy(app.policy, executionMode);
  } catch (e) {
    log.error(colors.red(`Error generating policy for app ${path}: ${e}`));
    throw e;
  }
}

/** What the regenerated policy starts from: the deployed policy, so a push keeps
 * settings the tracked file doesn't record. A first push has no deployed policy,
 * and then the file is what states one.
 *
 * The run identity comes along only when `claimsOnBehalfOf` says this push is
 * entitled to it — never from the file, and never from a pusher who may not
 * preserve one. The backend rewrites an unclaimed `on_behalf_of` to the pusher,
 * but `wmill` is regularly pointed at older servers, so the only identity that
 * goes on the wire is one this push is allowed to deploy under. */
export function basePolicy(
  localApp: any,
  deployedPolicy: Policy | undefined,
  claimsOnBehalfOf: boolean
): Policy | undefined {
  const stated = deployedPolicy ?? (localApp?.policy as Policy | undefined);
  if (!stated || claimsOnBehalfOf) {
    return stated;
  }
  const base: Policy = { ...stated };
  delete base.on_behalf_of;
  delete base.on_behalf_of_email;
  return base;
}

/** Claim the run-as identity the regenerated policy carries over from the
 * deployed app. Only a deployed identity may be claimed, never one the tracked
 * file states — a repo doesn't get to pick who an app runs as. Without the flag
 * the backend rewrites `on_behalf_of` to whoever is pushing, and it only honors
 * the flag for an admin or a `wm_deployers` member, so a caller who is neither
 * doesn't get to claim it here either. */
export function preserveOnBehalfOfFields(
  remotePath: string,
  deployedPolicy: Policy | undefined,
  permissionedAsContext: PermissionedAsContext | undefined
): { preserve_on_behalf_of?: boolean } {
  const onBehalfOf = deployedPolicy?.on_behalf_of;
  if (!permissionedAsContext?.userIsAdminOrDeployer || !onBehalfOf) {
    return {};
  }
  log.info(
    `Preserving ${deployedPolicy?.on_behalf_of_email ?? onBehalfOf} as permissioned_as for app ${remotePath}`
  );
  return { preserve_on_behalf_of: true };
}

/** The policy is written wholesale by the deploy, so the fields it does not
 * derive from the tracked sources have to survive the trip. The policy builder
 * has already recomputed what it can — the triggerables on both paths, plus the
 * S3 rules on the low-code one, which `updateRawAppPolicy` has no equivalent of
 * and so carries over. This sets the two left: the access mode, and the legacy
 * `triggerables`, which still grant execution (the backend folds them into
 * `triggerables_v2` at run time) and so are dropped rather than carried, or a
 * deployed app would keep being able to run runnables this push removed. */
export function finalizeDerivedPolicy(
  policy: Policy,
  executionMode: AppExecutionMode
) {
  policy.triggerables = undefined;
  policy.execution_mode = executionMode;
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

const APP_FOLDER_SUFFIXES = ["__raw_app", ".raw_app", "__app", ".app"] as const;

async function push(
  opts: GlobalOptions,
  filePath?: string,
  remotePath?: string
) {
  // Capture original CWD before resolveWorkspace, which may chdir to the
  // wmill.yaml root. We need it to resolve relative inputs and to derive
  // the remote path from the user's location when auto-inferring.
  const originalCwd = process.cwd();

  const workspace = await resolveWorkspace(opts);
  await requireLogin(opts);

  // Auto-infer file path from CWD when omitted
  if (!filePath) {
    filePath = originalCwd;
  }
  const absoluteFilePath = isAbsolute(filePath)
    ? filePath
    : pathResolve(originalCwd, filePath);

  // Detect app folder type (regular vs raw)
  const normalizedPath = absoluteFilePath.endsWith(SEP)
    ? absoluteFilePath.slice(0, -1)
    : absoluteFilePath;
  const dirName = basename(normalizedPath);
  const isRawAppByName =
    dirName.endsWith("__raw_app") || dirName.endsWith(".raw_app");
  const isAppByName = dirName.endsWith("__app") || dirName.endsWith(".app");

  let hasRawAppYaml = false;
  let hasAppYaml = false;
  try {
    await stat(normalizedPath + SEP + "raw_app.yaml");
    hasRawAppYaml = true;
  } catch { /* not a raw app */ }
  if (!hasRawAppYaml) {
    try {
      await stat(normalizedPath + SEP + "app.yaml");
      hasAppYaml = true;
    } catch { /* not an app */ }
  }

  if (!isRawAppByName && !isAppByName && !hasRawAppYaml && !hasAppYaml) {
    log.error(
      colors.red(
        `'${filePath}' is not an app folder (no app.yaml or raw_app.yaml, and not a *.app/*.raw_app folder).`
      )
    );
    return;
  }

  // Auto-infer remote path from the folder location relative to wmill.yaml root
  if (!remotePath) {
    const wmillYamlPath = getWmillYamlPath();
    if (!wmillYamlPath) {
      log.error(
        colors.red(
          "Could not infer remote path: no wmill.yaml found. Run 'wmill init' or pass <remote_path> explicitly."
        )
      );
      return;
    }
    // After resolveWorkspace, process.cwd() is the wmill.yaml dir
    const wmillRoot = process.cwd();
    let inferred = pathRelative(wmillRoot, normalizedPath).replaceAll(SEP, "/");
    if (inferred.startsWith("..") || isAbsolute(inferred)) {
      log.error(
        colors.red(
          `Could not infer remote path: '${filePath}' is outside the wmill.yaml root (${wmillRoot}). Move the folder under the root or pass <remote_path> explicitly.`
        )
      );
      return;
    }
    for (const suffix of APP_FOLDER_SUFFIXES) {
      if (inferred.endsWith(suffix)) {
        inferred = inferred.slice(0, -suffix.length);
        break;
      }
    }
    if (!inferred) {
      log.error(
        colors.red(
          "Could not infer remote path: app folder is at the wmill.yaml root. Pass <remote_path> explicitly."
        )
      );
      return;
    }
    if (
      !inferred.startsWith("u/") &&
      !inferred.startsWith("g/") &&
      !inferred.startsWith("f/")
    ) {
      log.error(
        colors.red(
          `Could not infer remote path: '${inferred}' is not under u/, g/, or f/. Move the app under one of these prefixes or pass <remote_path> explicitly.`
        )
      );
      return;
    }
    remotePath = inferred;
    log.info(colors.gray(`Inferred remote path: ${remotePath}`));
  }

  if (!validatePath(remotePath)) {
    return;
  }

  if (isRawAppByName || hasRawAppYaml) {
    const { pushRawApp } = await import("./raw_apps.ts");
    const merged = await mergeConfigWithConfigFile(opts);
    await pushRawApp(
      workspace.workspaceId,
      remotePath,
      absoluteFilePath,
      undefined,
      merged.defaultTs,
      await buildPermissionedAsContext(
        workspace.workspaceId,
        await readEffectiveSyncBehavior(opts, workspace),
      ),
    );
    log.info(colors.bold.underline.green("Raw app pushed"));
  } else {
    await pushApp(
      workspace.workspaceId,
      remotePath,
      absoluteFilePath,
      undefined,
      await buildPermissionedAsContext(
        workspace.workspaceId,
        await readEffectiveSyncBehavior(opts, workspace),
      ),
    );
    log.info(colors.bold.underline.green("App pushed"));
  }
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
  .command(
    "push",
    "push a local app. With no args, infers the app from the current directory and the remote path from its location relative to wmill.yaml."
  )
  .arguments("[file_path:string] [remote_path:string]")
  .action(push as any)
  .command("dev", devCommand)
  .command("lint", lintCommand)
  .command("bundle", bundleCommand)
  .command("new", newCommand)
  .command("generate-agents", generateAgentsCommand)
  .command(
    "generate-locks",
    'DEPRECATED: re-generate app lockfiles. Use "wmill generate-metadata" instead.'
  )
  // Deprecated compatibility command. Keep it working for older repos, but
  // exclude it from generated system prompt docs.
  // @deprecated use `wmill generate-metadata`
  .arguments("[app_folder:string]")
  .option("--yes", "Skip confirmation prompt")
  .option("--dry-run", "Perform a dry run without making changes")
  .option(
    "--default-ts <runtime:string>",
    "Default TypeScript runtime (bun or deno)"
  )
  .action(async (opts: any, appFolder: string | undefined) => {
    log.warn(
      colors.yellow('This command is deprecated. Use "wmill generate-metadata" instead.')
    );
    const { generateLocksCommand } = await import("./app_metadata.ts");
    await generateLocksCommand(opts, appFolder);
  })
  .command(
    "set-permissioned-as",
    "Set the on_behalf_of_email for an app (requires admin or wm_deployers group)"
  )
  .arguments("<path:string> <email:string>")
  .action((async (opts: any, appPath: string, email: string) => {
    const workspace = await resolveWorkspace(opts);
    await requireLogin(opts);

    const { lookupUsernameByEmail } = await import("../../core/permissioned_as.ts");
    const cache = new Map<string, { username: string; email: string }>();
    const username = await lookupUsernameByEmail(workspace.workspaceId, email, cache);

    const remote = await wmill.getAppByPath({
      workspace: workspace.workspaceId,
      path: appPath,
    });
    if (!remote) throw new Error(`App ${appPath} not found`);

    // Only spread remote.policy — spreading the full remote object would include
    // `value` and trigger a new app version on every call. EditApp has all-Option
    // fields, so a minimal body only updates the policy column.
    await wmill.updateApp({
      workspace: workspace.workspaceId,
      path: appPath,
      requestBody: {
        policy: {
          ...(remote.policy as any),
          on_behalf_of: `u/${username}`,
          on_behalf_of_email: email,
        } as any,
        preserve_on_behalf_of: true,
        // Preserve any user draft at this path (see backend skip_draft_deletion).
        skip_draft_deletion: true,
      },
    });
    log.info(colors.green(`Updated permissioned_as for app ${appPath} to ${email}`));
  }) as any);

export default command;
