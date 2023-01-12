import { decoverto } from "./decoverto.ts";
import { FlowFile } from "./flow.ts";
import { ResourceTypeFile } from "./resource-type.ts";
import { ResourceFile } from "./resource.ts";
import { ScriptFile } from "./script.ts";
import { VariableFile } from "./variable.ts";

export interface Resource {
  push(workspace: string, remotePath: string): Promise<void>;
}

export interface PushDiffs {
  pushDiffs(
    workspace: string,
    remotePath: string,
    diffs: Difference[],
  ): Promise<void>;
}

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

export function setValueByPath(
  obj: any,
  path: (string | number)[],
  value: any,
) {
  let i;
  for (i = 0; i < path.length - 1; i++) {
    obj = obj[path[i]];
  }
  obj[path[i]] = value;
}

export type GlobalOptions = {
  workspace: string | undefined;
  token: string | undefined;
};

export function inferTypeFromPath(
  path: string,
  obj: any,
): ScriptFile | VariableFile | FlowFile | ResourceFile | ResourceTypeFile {
  const typeEnding = path.split(".")[-1];
  if (typeEnding === "script") {
    return decoverto.type(ScriptFile).plainToInstance(obj);
  } else if (typeEnding === "variable") {
    return decoverto.type(VariableFile).plainToInstance(obj);
  } else if (typeEnding === "flow") {
    return decoverto.type(FlowFile).plainToInstance(obj);
  } else if (typeEnding === "resource") {
    return decoverto.type(ResourceFile).plainToInstance(obj);
  } else if (typeEnding === "resource-type") {
    return decoverto.type(ResourceTypeFile).plainToInstance(obj);
  } else {
    throw new Error("Could not infer type of path " + path);
  }
}
