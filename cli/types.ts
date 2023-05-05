// deno-lint-ignore-file no-explicit-any

import { colors, path } from "./deps.ts";
import { pushApp } from "./apps.ts";
import {
  parse as yamlParse,
  stringify as yamlStringify,
} from "https://deno.land/std@0.184.0/yaml/mod.ts";
import { equal } from "https://deno.land/x/equal@v1.5.0/equal.ts";
import { pushFolder } from "./folder.ts";
import { pushScript } from "./script.ts";
import { pushFlow } from "./flow.ts";
import { pushResource } from "./resource.ts";
import { pushResourceType } from "./resource-type.ts";
import { pushVariable } from "./variable.ts";
import * as Diff from "npm:diff";

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
    const eq = equal(subset[key], superset[key]);
    if (!eq) {
      console.log(
        `Found diff for ${key}:`,
        showDiff(yamlStringify(subset[key]), yamlStringify(superset[key]))
      );
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
      // print white if unchanged
      finalString += `\x1b[37m${part.value}\x1b[0m`;
    }
  }
  console.log(finalString);
}

export function showConflict(path: string, local: string, remote: string) {
  console.log(colors.yellow(`- ${path}`));
  showDiff(local, remote);
  console.log("\x1b[31mlocal\x1b[31m - \x1b[32mremote\x1b[32m");
  console.log();
}

export function pushObj(
  workspace: string,
  p: string,
  befObj: any,
  newObj: any,
  plainSecrets: boolean
) {
  const typeEnding = getTypeStrFromPath(p);

  if (typeEnding === "app") {
    pushApp(workspace, p, befObj, newObj);
  } else if (typeEnding === "folder") {
    pushFolder(workspace, p, befObj, newObj);
  } else if (typeEnding === "script") {
    pushScript(workspace, p, befObj, newObj);
  } else if (typeEnding === "variable") {
    pushVariable(workspace, p, befObj, newObj, plainSecrets);
  } else if (typeEnding === "flow") {
    pushFlow(workspace, p, befObj, newObj);
  } else if (typeEnding === "resource") {
    pushResource(workspace, p, befObj, newObj);
  } else if (typeEnding === "resource-type") {
    pushResourceType(workspace, p, befObj, newObj);
  } else {
    throw new Error("infer type unreachable");
  }
}

export function parseFromPath(p: string, content: string): any {
  return p.endsWith(".yaml") ? yamlParse(content) : JSON.parse(content);
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
  | "app" {
  const parsed = path.parse(p);
  if (
    parsed.ext == ".go" ||
    parsed.ext == ".ts" ||
    parsed.ext == ".sh" ||
    parsed.ext == ".py"
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
    typeEnding === "flow" ||
    typeEnding === "resource" ||
    typeEnding === "resource-type" ||
    typeEnding === "app"
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
