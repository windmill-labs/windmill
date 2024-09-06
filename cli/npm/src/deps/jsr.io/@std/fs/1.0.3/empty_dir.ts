// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
import * as dntShim from "../../../../../_dnt.shims.js";

import { join } from "../../path/1.0.4/join.js";
import { toPathString } from "./_to_path_string.js";

/**
 * Asynchronously ensures that a directory is empty.
 *
 * If the directory does not exist, it is created. The directory itself is not
 * deleted.
 *
 * Requires `--allow-read` and `--allow-write` permissions.
 *
 * @see {@link https://docs.deno.com/runtime/manual/basics/permissions#file-system-access}
 * for more information on Deno's permissions system.
 *
 * @param dir The path of the directory to empty, as a string or URL.
 *
 * @returns A void promise that resolves once the directory is empty.
 *
 * @example Usage
 * ```ts no-eval
 * import { emptyDir } from "@std/fs/empty-dir";
 *
 * await emptyDir("./foo");
 * ```
 */
export async function emptyDir(dir: string | URL) {
  try {
    const items = await Array.fromAsync(dntShim.Deno.readDir(dir));

    await Promise.all(items.map((item) => {
      if (item && item.name) {
        const filepath = join(toPathString(dir), item.name);
        return dntShim.Deno.remove(filepath, { recursive: true });
      }
    }));
  } catch (err) {
    if (!(err instanceof dntShim.Deno.errors.NotFound)) {
      throw err;
    }

    // if not exist. then create it
    await dntShim.Deno.mkdir(dir, { recursive: true });
  }
}

/**
 * Synchronously ensures that a directory is empty deletes the directory
 * contents it is not empty.
 *
 * If the directory does not exist, it is created. The directory itself is not
 * deleted.
 *
 * Requires `--allow-read` and `--allow-write` permissions.
 *
 * @see {@link https://docs.deno.com/runtime/manual/basics/permissions#file-system-access}
 * for more information on Deno's permissions system.
 *
 * @param dir The path of the directory to empty, as a string or URL.
 *
 * @returns A void value that returns once the directory is empty.
 *
 * @example Usage
 * ```ts no-eval
 * import { emptyDirSync } from "@std/fs/empty-dir";
 *
 * emptyDirSync("./foo");
 * ```
 */
export function emptyDirSync(dir: string | URL) {
  try {
    const items = [...dntShim.Deno.readDirSync(dir)];

    // If the directory exists, remove all entries inside it.
    while (items.length) {
      const item = items.shift();
      if (item && item.name) {
        const filepath = join(toPathString(dir), item.name);
        dntShim.Deno.removeSync(filepath, { recursive: true });
      }
    }
  } catch (err) {
    if (!(err instanceof dntShim.Deno.errors.NotFound)) {
      throw err;
    }
    // if not exist. then create it
    dntShim.Deno.mkdirSync(dir, { recursive: true });
  }
}
