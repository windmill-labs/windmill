// deno-lint-ignore-file no-explicit-any

import {
  Diff,
  SEP,
  colors,
  log,
  path,
  yamlParseContent,
  yamlStringify,
} from "./deps.ts";
import { pushApp } from "./apps.ts";
import { pushFolder } from "./folder.ts";
import { pushFlow } from "./flow.ts";
import { pushResource } from "./resource.ts";
import { pushResourceType } from "./resource-type.ts";
import { pushVariable } from "./variable.ts";
import { yamlOptions } from "./sync.ts";
import { showDiffs } from "./main.ts";
import { deepEqual, isFileResource } from "./utils.ts";
import { pushSchedule } from "./schedule.ts";
import { pushWorkspaceUser } from "./user.ts";
import { pushGroup } from "./user.ts";
import { pushWorkspaceSettings, pushWorkspaceKey } from "./settings.ts";
import { pushTrigger } from "./trigger.ts";

export interface DifferenceCreate {
  type: "CREATE";
  path: (string | number)[];
  value: any;
}

export interface DifferenceRemove {
  type: "REMOVE";
  path: (string | number)[];
  oldValue: any;
}

export interface DifferenceChange {
  type: "CHANGE";
  path: (string | number)[];
  value: any;
  oldValue: any;
}

export type Difference = DifferenceCreate | DifferenceRemove | DifferenceChange;

export type GlobalOptions = {
  baseUrl: string | undefined;
  workspace: string | undefined;
  token: string | undefined;
};

export function isSuperset(
  subset: Record<string, any>,
  superset: Record<string, any>
): boolean {
  return Object.keys(subset).every((key) => {
    const eq = deepEqual(subset[key], superset[key]);
    if (!eq && showDiffs) {
      const sub = subset[key];
      const supers = superset[key];
      if (!supers) {
        log.info(`Key ${key} not found in remote`);
      } else {
        log.info(`Found diff for ${key}:`);
        showDiff(
          yamlStringify(sub, yamlOptions),
          yamlStringify(supers, yamlOptions)
        );
      }
    }
    return eq;
  });
}

export function showDiff(local: string, remote: string) {
  let finalString = "";
  if (local?.length > 20000 || remote?.length > 20000) {
    log.info("Diff too large to display");
    return;
  }

  for (const part of Diff.diffLines(local ?? "", remote ?? "")) {
    if (part.removed) {
      // print red if removed without newline
      finalString += `\x1b[31m${part.value}\x1b[0m`;
    } else if (part.added) {
      // print green if added
      finalString += `\x1b[32m${part.value}\x1b[0m`;
    } else {
      let lines = part.value.split("\n");

      if (lines.length > 4) {
        lines = lines.slice(0, 2);
        lines.push("...");
        lines = lines.concat(part.value.split("\n").slice(-2));
      }
      // print white if unchanged
      finalString += `\x1b[37m${lines.join("\n")}\x1b[0m`;
    }
  }
  log.info(finalString);
}

export function showConflict(path: string, local: string, remote: string) {
  log.info(colors.yellow(`- ${path}`));
  showDiff(local, remote);
  log.info("\x1b[31mlocal\x1b[31m - \x1b[32mremote\x1b[32m");
  log.info("\n");
}

export async function pushObj(
  workspace: string,
  p: string,
  befObj: any,
  newObj: any,
  plainSecrets: boolean,
  alreadySynced: string[],
  message?: string
) {
  const typeEnding = getTypeStrFromPath(p);

  if (typeEnding === "app") {
    const appName = p.split(".app" + SEP)[0];
    await pushApp(workspace, appName, appName + ".app", message);
  } else if (typeEnding === "folder") {
    await pushFolder(workspace, p, befObj, newObj);
  } else if (typeEnding === "variable") {
    await pushVariable(workspace, p, befObj, newObj, plainSecrets);
  } else if (typeEnding === "flow") {
    const flowName = p.split(".flow" + SEP)[0];
    await pushFlow(workspace, flowName, flowName + ".flow", message);
  } else if (typeEnding === "resource") {
    if (!alreadySynced.includes(p)) {
      alreadySynced.push(p);
      await pushResource(workspace, p, befObj, newObj);
    }
  } else if (typeEnding === "resource-type") {
    await pushResourceType(workspace, p, befObj, newObj);
  } else if (typeEnding === "schedule") {
    await pushSchedule(workspace, p, befObj, newObj);
  } else if (typeEnding === "http_trigger") {
    await pushTrigger("http", workspace, p, befObj, newObj);
  } else if (typeEnding === "websocket_trigger") {
    await pushTrigger("websocket", workspace, p, befObj, newObj);
  } else if (typeEnding === "kafka_trigger") {
    await pushTrigger("kafka", workspace, p, befObj, newObj);
  } else if (typeEnding === "nats_trigger") {
    await pushTrigger("nats", workspace, p, befObj, newObj);
  } else if (typeEnding === "postgres_trigger") {
    await pushTrigger("postgres", workspace, p, befObj, newObj);
  } else if (typeEnding === "mqtt_trigger") {
    await pushTrigger("mqtt", workspace, p, befObj, newObj);
  } else if (typeEnding === "sqs_trigger") {
    await pushTrigger("sqs", workspace, p, befObj, newObj);
  } else if (typeEnding === "user") {
    await pushWorkspaceUser(workspace, p, befObj, newObj);
  } else if (typeEnding === "group") {
    await pushGroup(workspace, p, befObj, newObj);
  } else if (typeEnding === "settings") {
    await pushWorkspaceSettings(workspace, p, befObj, newObj);
  } else if (typeEnding === "encryption_key") {
    await pushWorkspaceKey(workspace, p, befObj, newObj);
  } else {
    throw new Error(
      `The item ${p} has an unrecognized type ending ${typeEnding}`
    );
  }
}

export function parseFromPath(p: string, content: string): any {
  return p.endsWith(".yaml")
    ? yamlParseContent(p, content)
    : p.endsWith(".json")
    ? JSON.parse(content)
    : content;
}
export function parseFromFile(p: string): any {
  if (p.endsWith(".json")) {
    return JSON.parse(Deno.readTextFileSync(p));
  } else if (p.endsWith(".yaml") || p.endsWith(".yml")) {
    return yamlParseContent(p, Deno.readTextFileSync(p));
  } else {
    throw new Error("Could not read file " + p);
  }
}
export function getTypeStrFromPath(
  p: string
):
  | "script"
  | "variable"
  | "flow"
  | "resource"
  | "resource-type"
  | "folder"
  | "app"
  | "schedule"
  | "http_trigger"
  | "websocket_trigger"
  | "kafka_trigger"
  | "nats_trigger"
  | "postgres_trigger"
  | "mqtt_trigger"
  | "sqs_trigger"
  | "user"
  | "group"
  | "settings"
  | "encryption_key" {
  if (p.includes(".flow" + SEP)) {
    return "flow";
  }
  if (p.includes(".app" + SEP)) {
    return "app";
  }
  const parsed = path.parse(p);
  if (
    parsed.ext == ".go" ||
    parsed.ext == ".ts" ||
    parsed.ext == ".sh" ||
    parsed.ext == ".py" ||
    parsed.ext == ".sql" ||
    parsed.ext == ".gql" ||
    parsed.ext == ".ps1" ||
    parsed.ext == ".js" ||
    parsed.ext == ".php" ||
    parsed.ext == ".rs" ||
    parsed.ext == ".cs" ||
    parsed.ext == ".nu" ||
    parsed.ext == ".java" ||
    parsed.ext == ".rb" ||
		// for related places search: ADD_NEW_LANG 
    (parsed.ext == ".yml" && parsed.name.split(".").pop() == "playbook")
  ) {
    return "script";
  }

  if (parsed.name === "folder.meta") {
    return "folder";
  }

  const typeEnding = parsed.name.split(".").at(-1);
  if (
    typeEnding === "script" ||
    typeEnding === "variable" ||
    typeEnding === "resource" ||
    typeEnding === "resource-type" ||
    typeEnding === "app" ||
    typeEnding === "schedule" ||
    typeEnding === "http_trigger" ||
    typeEnding === "websocket_trigger" ||
    typeEnding === "kafka_trigger" ||
    typeEnding === "nats_trigger" ||
    typeEnding === "postgres_trigger" ||
    typeEnding === "mqtt_trigger" ||
    typeEnding === "sqs_trigger" ||
    typeEnding === "user" ||
    typeEnding === "group" ||
    typeEnding === "settings" ||
    typeEnding === "encryption_key"
  ) {
    return typeEnding;
  } else {
    if (isFileResource(p)) {
      return "resource";
    }
    throw new Error("Could not infer type of path " + JSON.stringify(parsed));
  }
}

export function removeType(str: string, type: string) {
  if (
    !str.endsWith("." + type + ".yaml") &&
    !str.endsWith("." + type + ".json")
  ) {
    throw new Error(str + " does not end with ." + type + ".(yaml|json)");
  }
  return str.slice(0, str.length - type.length - 6);
}

export function removePathPrefix(str: string, prefix: string) {
  if (!str.startsWith(prefix + "/")) {
    throw new Error(str + " does not start with " + prefix);
  }
  return str.slice(prefix.length + 1);
}
