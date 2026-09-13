import * as wmill from "../../gen/services.gen.ts";
import * as log from "./log.ts";
import { colors } from "@cliffy/ansi/colors";
import { Confirm } from "@cliffy/prompt/confirm";
import { getTypeStrFromPath } from "../types.ts";
import {
  extractFolderPath,
  isAppFolderMetadataFile,
  isRawAppFolderMetadataFile,
} from "../utils/resource_folders.ts";
import { deploysWithRawApp } from "../utils/app_files.ts";
import { parseSyncBehavior } from "./conf.ts";

export interface PermissionedAsContext {
  userCache: Map<string, { username: string; email: string }>;
  userIsAdminOrDeployer: boolean;
  userEmail: string;
}

/**
 * The whole-tree `sync push` and the single-item `push` commands must resolve
 * ownership the same way, so both build the context here: a push that leaves it
 * undefined reassigns `permissioned_as` / `on_behalf_of` to whoever ran it.
 * Undefined below syncBehavior v1, where that reassignment is the contract, and
 * for a caller who is neither admin nor in `wm_deployers` the backend enforces
 * it anyway — the flag on the context is what keeps the CLI from claiming
 * otherwise.
 */
export async function buildPermissionedAsContext(
  workspace: string,
  syncBehavior: string | number | undefined
): Promise<PermissionedAsContext | undefined> {
  if (parseSyncBehavior(syncBehavior) < 1) {
    return undefined;
  }
  const user = await wmill.whoami({ workspace });
  const userIsAdminOrDeployer =
    user.is_admin || (user.groups ?? []).includes("wm_deployers");
  log.debug(
    `permissioned_as: user=${user.email}, is_admin=${user.is_admin}, groups=${JSON.stringify(user.groups)}, isAdminOrDeployer=${userIsAdminOrDeployer}`
  );
  return {
    userCache: new Map(),
    userIsAdminOrDeployer,
    userEmail: user.email,
  };
}

async function ensureUserCache(
  workspace: string,
  cache: Map<string, { username: string; email: string }>
): Promise<void> {
  if (cache.size > 0) return;
  const users = await wmill.listUsers({ workspace });
  for (const user of users) {
    cache.set(user.username, { username: user.username, email: user.email });
    cache.set(user.email, { username: user.username, email: user.email });
  }
}

/**
 * The address a workspace member's username belongs to, or `undefined` when the workspace has no
 * such member. Absence is an answer, not a failure: a principal naming no `usr` row is an
 * instance-level identity (a superadmin acting outside their workspaces), which is not
 * workspace-scoped and so means the same thing in every workspace.
 *
 * Reads `usr` through `listUsers`, so it is never the cached address a derived read would give.
 */
export async function lookupEmailByUsername(
  workspace: string,
  username: string,
  cache: Map<string, { username: string; email: string }>
): Promise<string | undefined> {
  await ensureUserCache(workspace, cache);
  return cache.get(username)?.email;
}

export async function lookupUsernameByEmail(
  workspace: string,
  email: string,
  cache: Map<string, { username: string; email: string }>
): Promise<string> {
  await ensureUserCache(workspace, cache);
  const entry = cache.get(email);
  if (!entry) {
    throw new Error(
      `Could not find username for email '${email}' in workspace. ` +
        `Make sure the user exists in the workspace.`
    );
  }
  return entry.username;
}

export interface Change {
  name: "edited" | "added" | "deleted";
  path: string;
  before?: string;
  after?: string;
  content?: string;
}

function contentHasOnBehalfOf(content: string, typeStr: string): boolean {
  if (typeStr === "script") {
    return (
      !!content.match(/has_on_behalf_of:\s*(true)/) ||
      !!content.match(/on_behalf_of_email:\s*["']?([^\s"']+)["']?/)
    );
  }
  if (typeStr === "flow") {
    return !!content.match(/has_on_behalf_of:\s*(true)/);
  }
  return false;
}

type AppTypeStr = "app" | "raw_app";

function isAppTypeStr(typeStr: string): typeStr is AppTypeStr {
  return typeStr === "app" || typeStr === "raw_app";
}

/** The app folder a file belongs to. `isAppFolderMetadataFile` and its raw twin
 * match a literal `/`, unlike `extractFolderPath` — so normalize before either,
 * or a Windows path takes a different branch from the same file on Linux. */
function appFolderOf(path: string, typeStr: AppTypeStr): string {
  return extractFolderPath(path, typeStr) ?? path;
}

function toPosix(path: string): string {
  return path.replaceAll("\\", "/");
}

/** App folders whose own metadata file is being added or deleted, which is how a
 * whole app arrives or goes rather than being redeployed. Neither takes an owner
 * over: a create has none yet, and a delete leaves none behind. */
function appsArrivingOrLeaving(changes: Change[]): Set<string> {
  const folders = new Set<string>();
  for (const change of changes) {
    if (change.name === "edited") continue;
    const path = toPosix(change.path);
    if (!isAppFolderMetadataFile(path) && !isRawAppFolderMetadataFile(path)) {
      continue;
    }
    let typeStr: string;
    try {
      typeStr = getTypeStrFromPath(path);
    } catch {
      continue;
    }
    if (isAppTypeStr(typeStr)) folders.add(appFolderOf(path, typeStr));
  }
  return folders;
}

export async function preCheckPermissionedAs(
  changes: Change[],
  userEmail: string,
  userIsAdminOrDeployer: boolean,
  acceptOverride: boolean,
  isInteractive: boolean
): Promise<void> {
  // Admins and wm_deployers can always preserve ownership via the CLI's
  // preserve-on-update path, so nothing would silently change for them.
  if (userIsAdminOrDeployer) return;

  const wouldChangeItems: { path: string; currentOwner: string }[] = [];
  const addItem = (item: { path: string; currentOwner: string }) => {
    if (!wouldChangeItems.some((i) => i.path === item.path)) {
      wouldChangeItems.push(item);
    }
  };
  const arrivingOrLeaving = appsArrivingOrLeaving(changes);

  for (const change of changes) {
    let typeStr: string;
    try {
      typeStr = getTypeStrFromPath(change.path);
    } catch {
      continue;
    }

    // An app is redeployed whole by any change to any of the files it actually
    // sends — added, edited or deleted alike — so its policy is rewritten
    // regardless of what the file holds. Settled here, before the content the
    // other kinds parse to find their owner, which an app has none of to parse.
    if (isAppTypeStr(typeStr)) {
      const path = toPosix(change.path);
      const folder = appFolderOf(path, typeStr);
      if (
        !arrivingOrLeaving.has(folder) &&
        (typeStr === "app" || deploysWithRawApp(path.slice(folder.length)))
      ) {
        addItem({ path: folder, currentOwner: "(app policy owner)" });
      }
      continue;
    }

    if (change.name === "added") {
      const content = change.content;
      if (!content) continue;

      const isScriptMeta =
        typeStr === "script" &&
        (change.path.endsWith(".script.yaml") ||
          change.path.endsWith(".script.json"));
      const isFlowMeta =
        typeStr === "flow" &&
        (change.path.endsWith("flow.yaml") ||
          change.path.endsWith("flow.json"));

      if (
        (isScriptMeta || isFlowMeta) &&
        contentHasOnBehalfOf(content, typeStr)
      ) {
        const label =
          typeStr === "script" ? "(script owner)" : "(flow owner)";
        wouldChangeItems.push({ path: change.path, currentOwner: label });
      }
      continue;
    }

    if (change.name !== "edited") continue;

    const beforeContent = change.before;
    if (!beforeContent) continue;

    let currentOwner: string | undefined;

    if (typeStr === "script") {
      if (
        change.path.endsWith(".script.yaml") ||
        change.path.endsWith(".script.json")
      ) {
        const hasOboMatch = beforeContent.match(/has_on_behalf_of:\s*(true)/);
        if (hasOboMatch) {
          currentOwner = "(script owner)";
        } else {
          const emailMatch = beforeContent.match(
            /on_behalf_of_email:\s*["']?([^\s"']+)["']?/
          );
          if (emailMatch) {
            currentOwner = emailMatch[1];
          }
        }
      }
    } else if (typeStr === "flow") {
      if (
        change.path.endsWith("flow.yaml") ||
        change.path.endsWith("flow.json")
      ) {
        const hasOboMatch = beforeContent.match(/has_on_behalf_of:\s*(true)/);
        if (hasOboMatch) {
          wouldChangeItems.push({
            path: change.path,
            currentOwner: "(flow owner)",
          });
        }
      }
      continue;
    } else if (typeStr === "schedule") {
      const match = beforeContent.match(
        /email:\s*["']?([^\s"']+)["']?/
      );
      if (match) {
        currentOwner = match[1];
      }
    } else if (typeStr.endsWith("_trigger")) {
      wouldChangeItems.push({
        path: change.path,
        currentOwner: "(trigger owner)",
      });
      continue;
    }

    if (currentOwner && currentOwner !== userEmail) {
      wouldChangeItems.push({ path: change.path, currentOwner });
    }
  }

  if (wouldChangeItems.length === 0) return;

  const itemList = wouldChangeItems
    .map((item) => `  - ${item.path} (current owner: ${item.currentOwner})`)
    .join("\n");

  const message =
    `You are not an admin or member of 'wm_deployers'. The following ${wouldChangeItems.length} item(s) ` +
    `will have their permissioned_as/email changed to your user (${userEmail}):\n${itemList}`;

  if (acceptOverride) {
    log.warn(colors.yellow(`Warning: ${message}`));
    return;
  }

  if (isInteractive) {
    log.warn(colors.yellow(message));
    const proceed = await Confirm.prompt({
      message:
        "Do you want to proceed? (use --accept-overriding-permissioned-as-with-self to skip this prompt)",
      default: false,
    });
    if (!proceed) {
      log.info("Push cancelled.");
      process.exit(0);
    }
  } else {
    log.error(
      colors.red(
        `${message}\n\nUse --accept-overriding-permissioned-as-with-self to proceed anyway.`
      )
    );
    process.exit(1);
  }
}
