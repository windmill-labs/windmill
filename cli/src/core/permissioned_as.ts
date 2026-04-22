import * as wmill from "../../gen/services.gen.ts";
import * as log from "./log.ts";
import { colors } from "@cliffy/ansi/colors";
import { Confirm } from "@cliffy/prompt/confirm";
import { getTypeStrFromPath } from "../types.ts";

export interface PermissionedAsContext {
  userCache: Map<string, { username: string; email: string }>;
  userIsAdminOrDeployer: boolean;
  userEmail: string;
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

  for (const change of changes) {
    let typeStr: string;
    try {
      typeStr = getTypeStrFromPath(change.path);
    } catch {
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
      } else if (typeStr === "app") {
        wouldChangeItems.push({
          path: change.path,
          currentOwner: "(app policy owner)",
        });
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
    } else if (typeStr === "app") {
      wouldChangeItems.push({
        path: change.path,
        currentOwner: "(app policy owner)",
      });
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
