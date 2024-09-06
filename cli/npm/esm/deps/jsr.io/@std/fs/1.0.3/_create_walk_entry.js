// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// Copyright the Browserify authors. MIT License.
import * as dntShim from "../../../../../_dnt.shims.js";
import { basename } from "../../path/1.0.4/basename.js";
import { normalize } from "../../path/1.0.4/normalize.js";
import { toPathString } from "./_to_path_string.js";
/** Create {@linkcode WalkEntry} for the `path` synchronously. */
export function createWalkEntrySync(path) {
    path = toPathString(path);
    path = normalize(path);
    const name = basename(path);
    const info = dntShim.Deno.statSync(path);
    return {
        path,
        name,
        isFile: info.isFile,
        isDirectory: info.isDirectory,
        isSymlink: info.isSymlink,
    };
}
/** Create {@linkcode WalkEntry} for the `path` asynchronously. */
export async function createWalkEntry(path) {
    path = toPathString(path);
    path = normalize(path);
    const name = basename(path);
    const info = await dntShim.Deno.stat(path);
    return {
        path,
        name,
        isFile: info.isFile,
        isDirectory: info.isDirectory,
        isSymlink: info.isSymlink,
    };
}
