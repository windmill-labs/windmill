// deno-lint-ignore-file no-explicit-any

import { colors, log, path, yamlParse, yamlStringify } from "./deps.ts";
import { pushApp } from "./apps.ts";
import { pushFolder } from "./folder.ts";
import { pushFlow } from "./flow.ts";
import { pushResource } from "./resource.ts";
import { pushResourceType } from "./resource-type.ts";
import { pushVariable } from "./variable.ts";
import * as Diff from "npm:diff";
import { yamlOptions } from "./sync.ts";
import { showDiffs } from "./main.ts";
import { deepEqual } from "./utils.ts";
import { pushSchedule } from "./schedule.ts";

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
  for (const part of Diff.diffLines(local, remote)) {
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

export function pushObj(
  workspace: string,
  p: string,
  befObj: any,
  newObj: any,
  plainSecrets: boolean,
  checkForCreate: boolean
) {
  const typeEnding = getTypeStrFromPath(p);

  if (typeEnding === "app") {
    pushApp(workspace, p, befObj, newObj, checkForCreate);
  } else if (typeEnding === "folder") {
    pushFolder(workspace, p, befObj, newObj, checkForCreate);
  } else if (typeEnding === "variable") {
    pushVariable(workspace, p, befObj, newObj, plainSecrets, checkForCreate);
  } else if (typeEnding === "flow") {
    const flowName = p.split(".flow/")[0];
    pushFlow(workspace, flowName, flowName + ".flow");
  } else if (typeEnding === "resource") {
    pushResource(workspace, p, befObj, newObj, checkForCreate);
  } else if (typeEnding === "resource-type") {
    pushResourceType(workspace, p, befObj, newObj, checkForCreate);
  } else if (typeEnding === "schedule") {
    pushSchedule(workspace, p, befObj, newObj, checkForCreate);
  } else {
    throw new Error("infer type unreachable");
  }
}

export function parseFromPath(p: string, content: string): any {
  return p.endsWith(".yaml")
    ? yamlParse(content)
    : p.endsWith(".json")
    ? JSON.parse(content)
    : content;
}
export function parseFromFile(p: string): any {
  if (p.endsWith(".json")) {
    return JSON.parse(Deno.readTextFileSync(p));
  } else if (p.endsWith(".yaml") || p.endsWith(".yml")) {
    return yamlParse(Deno.readTextFileSync(p));
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
  | "schedule" {
  if (p.includes(".flow/")) {
    return "flow";
  }
  const parsed = path.parse(p);
  if (
    parsed.ext == ".go" ||
    parsed.ext == ".ts" ||
    parsed.ext == ".sh" ||
    parsed.ext == ".py" ||
    parsed.ext == ".sql"
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
    typeEnding === "schedule"
  ) {
    return typeEnding;
  } else {
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
