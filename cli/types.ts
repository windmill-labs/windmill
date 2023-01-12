import { decoverto } from "./decoverto.ts";
import { FlowFile } from "./flow.ts";
import { ResourceTypeFile } from "./resource-type.ts";
import { ResourceFile } from "./resource.ts";
import { ScriptFile } from "./ScriptFile";
import { VariableFile } from "./variable.ts";
import { path } from "./deps.ts";
import { FolderFile } from "./folder.ts";

// TODO: Remove this & replace with a "pull" that lets the object either pull the remote version or return undefined.
// Then combine those with diffing, which then gives the new push impl
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
  p: string,
  obj: any,
):
  | ScriptFile
  | VariableFile
  | FlowFile
  | ResourceFile
  | ResourceTypeFile
  | FolderFile {
  const parsed = path.parse(p);
  if (parsed.ext !== "json") {
    throw new Error("Cannot infer type of non-json file");
  }
  if (parsed.name === "folder.meta") {
    return decoverto.type(FolderFile).plainToInstance(obj);
  }
  const typeEnding = parsed.name.split(".")[-1];
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
