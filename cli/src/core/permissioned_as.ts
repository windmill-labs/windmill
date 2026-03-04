import { minimatch } from "minimatch";
import * as wmill from "../../gen/services.gen.ts";
import * as log from "./log.ts";
import { colors } from "@cliffy/ansi/colors";
import { Confirm } from "@cliffy/prompt/confirm";
import { getTypeStrFromPath } from "../types.ts";

export interface PermissionedAsRule {
  email: string;
  path_pattern: string;
}

export interface PermissionedAsContext {
  rules: PermissionedAsRule[];
  emailToUsernameCache: Map<string, string>;
  userIsAdminOrDeployer: boolean;
}

const KNOWN_RULE_FIELDS = new Set(["email", "path_pattern"]);

/**
 * Validates defaultPermissionedAs rules from wmill.yaml.
 * Throws on invalid rules so the user gets a clear error before any push happens.
 */
export function validatePermissionedAsRules(
  rules: unknown,
  source: string
): PermissionedAsRule[] {
  if (rules === undefined || rules === null) {
    return [];
  }
  if (!Array.isArray(rules)) {
    throw new Error(
      `Invalid defaultPermissionedAs in ${source}: expected an array of rules, got ${typeof rules}`
    );
  }

  const validated: PermissionedAsRule[] = [];

  for (let i = 0; i < rules.length; i++) {
    const rule = rules[i];
    const ruleLabel = `defaultPermissionedAs[${i}] in ${source}`;

    if (typeof rule !== "object" || rule === null) {
      throw new Error(`Invalid ${ruleLabel}: expected an object with 'email' and 'path_pattern' fields`);
    }

    // Check for unknown/misspelled fields
    const unknownFields = Object.keys(rule).filter((k) => !KNOWN_RULE_FIELDS.has(k));
    if (unknownFields.length > 0) {
      throw new Error(
        `Invalid ${ruleLabel}: unknown field(s) ${unknownFields.map((f) => `'${f}'`).join(", ")}. ` +
          `Valid fields are: 'email', 'path_pattern'`
      );
    }

    // Validate required fields
    if (typeof rule.email !== "string" || rule.email.trim() === "") {
      throw new Error(
        `Invalid ${ruleLabel}: 'email' is required and must be a non-empty string`
      );
    }
    if (typeof rule.path_pattern !== "string" || rule.path_pattern.trim() === "") {
      throw new Error(
        `Invalid ${ruleLabel}: 'path_pattern' is required and must be a non-empty string`
      );
    }

    // Validate glob pattern by testing it
    try {
      minimatch("test/path", rule.path_pattern);
    } catch (e) {
      throw new Error(
        `Invalid ${ruleLabel}: 'path_pattern' "${rule.path_pattern}" is not a valid glob pattern: ${
          e instanceof Error ? e.message : e
        }`
      );
    }

    validated.push({ email: rule.email, path_pattern: rule.path_pattern });
  }

  return validated;
}

/**
 * Resolves which email should be used for a new item based on defaultPermissionedAs rules.
 * Returns undefined if no rule matches.
 */
export function resolvePermissionedAsEmail(
  path: string,
  rules: PermissionedAsRule[]
): string | undefined {
  for (const rule of rules) {
    if (minimatch(path, rule.path_pattern)) {
      return rule.email;
    }
  }
  return undefined;
}

/**
 * Looks up a username by email using the workspace users API.
 * Results are cached in the provided map.
 */
export async function lookupUsernameByEmail(
  workspace: string,
  email: string,
  cache: Map<string, string>
): Promise<string> {
  if (cache.has(email)) {
    return cache.get(email)!;
  }

  // Populate entire cache from users list (only fetched once)
  if (cache.size === 0) {
    const users = await wmill.listUsers({ workspace });
    for (const user of users) {
      cache.set(user.email, user.username);
    }
  }

  const username = cache.get(email);
  if (!username) {
    throw new Error(
      `Could not find username for email '${email}' in workspace. ` +
        `Make sure the user exists in the workspace.`
    );
  }
  return username;
}

export interface Change {
  name: "edited" | "added" | "deleted";
  path: string;
  before?: string;
  after?: string;
  content?: string;
}

/**
 * Pre-checks whether items being pushed will have their permissioned_as/email changed
 * because the deploying user is not an admin or deployer.
 *
 * For admins/deployers, preserve flags will be sent with the API calls, so no warning is needed.
 * For non-admin/non-deployer users, the API will silently overwrite the owner to the deploying user.
 */
export async function preCheckPermissionedAs(
  changes: Change[],
  userEmail: string,
  userIsAdminOrDeployer: boolean,
  acceptOverride: boolean,
  isInteractive: boolean
): Promise<void> {
  if (userIsAdminOrDeployer) {
    return;
  }

  const wouldChangeItems: { path: string; currentOwner: string }[] = [];

  for (const change of changes) {
    if (change.name !== "edited") {
      continue;
    }

    let typeStr: string;
    try {
      typeStr = getTypeStrFromPath(change.path);
    } catch {
      continue;
    }

    const beforeContent = change.before;
    if (!beforeContent) continue;

    let currentOwner: string | undefined;

    if (typeStr === "script") {
      // Script metadata is in .script.yaml files
      if (
        change.path.endsWith(".script.yaml") ||
        change.path.endsWith(".script.json")
      ) {
        const match = beforeContent.match(
          /on_behalf_of_email:\s*["']?([^\s"']+)["']?/
        );
        if (match) {
          currentOwner = match[1];
        }
      }
    } else if (typeStr === "flow") {
      // Flow metadata may have on_behalf_of_email
      const match = beforeContent.match(
        /on_behalf_of_email:\s*["']?([^\s"']+)["']?/
      );
      if (match) {
        currentOwner = match[1];
      }
    } else if (typeStr === "app") {
      // Apps always have on_behalf_of set - any edited app will change owner
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
      const match = beforeContent.match(
        /email:\s*["']?([^\s"']+)["']?/
      );
      if (match) {
        currentOwner = match[1];
      }
    }

    if (currentOwner && currentOwner !== userEmail) {
      wouldChangeItems.push({ path: change.path, currentOwner });
    }
  }

  if (wouldChangeItems.length === 0) {
    return;
  }

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
      throw new Error("Push cancelled by user due to permissioned_as changes.");
    }
  } else {
    throw new Error(
      `${message}\n\nUse --accept-overriding-permissioned-as-with-self to proceed anyway.`
    );
  }
}
