// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
import * as dntShim from "../../../../../_dnt.shims.js";

import { dirname } from "../../path/1.0.4/dirname.js";
import { ensureDir, ensureDirSync } from "./ensure_dir.js";
import { getFileInfoType } from "./_get_file_info_type.js";
import { toPathString } from "./_to_path_string.js";

/**
 * Asynchronously ensures that the file exists.
 *
 * If the file already exists, this function does nothing. If the parent
 * directories for the file do not exist, they are created.
 *
 * Requires `--allow-read` and `--allow-write` permissions.
 *
 * @see {@link https://docs.deno.com/runtime/manual/basics/permissions#file-system-access}
 * for more information on Deno's permissions system.
 *
 * @param filePath The path of the file to ensure, as a string or URL.
 *
 * @returns A void promise that resolves once the file exists.
 *
 * @example Usage
 * ```ts no-eval
 * import { ensureFile } from "@std/fs/ensure-file";
 *
 * await ensureFile("./folder/targetFile.dat");
 * ```
 */
export async function ensureFile(filePath: string | URL): Promise<void> {
  try {
    // if file exists
    const stat = await dntShim.Deno.lstat(filePath);
    if (!stat.isFile) {
      throw new Error(
        `Failed to ensure file exists: expected 'file', got '${
          getFileInfoType(stat)
        }'`,
      );
    }
  } catch (err) {
    // if file not exists
    if (err instanceof dntShim.Deno.errors.NotFound) {
      // ensure dir exists
      await ensureDir(dirname(toPathString(filePath)));
      // create file
      await dntShim.Deno.writeFile(filePath, new Uint8Array());
      return;
    }

    throw err;
  }
}

/**
 * Synchronously ensures that the file exists.
 *
 * If the file already exists, this function does nothing. If the parent
 * directories for the file do not exist, they are created.
 *
 * Requires `--allow-read` and `--allow-write` permissions.
 *
 * @see {@link https://docs.deno.com/runtime/manual/basics/permissions#file-system-access}
 * for more information on Deno's permissions system.
 *
 * @param filePath The path of the file to ensure, as a string or URL.
 *
 * @returns A void value that returns once the file exists.
 *
 * @example Usage
 * ```ts no-eval
 * import { ensureFileSync } from "@std/fs/ensure-file";
 *
 * ensureFileSync("./folder/targetFile.dat");
 * ```
 */
export function ensureFileSync(filePath: string | URL): void {
  try {
    // if file exists
    const stat = dntShim.Deno.lstatSync(filePath);
    if (!stat.isFile) {
      throw new Error(
        `Failed to ensure file exists: expected 'file', got '${
          getFileInfoType(stat)
        }'`,
      );
    }
  } catch (err) {
    // if file not exists
    if (err instanceof dntShim.Deno.errors.NotFound) {
      // ensure dir exists
      ensureDirSync(dirname(toPathString(filePath)));
      // create file
      dntShim.Deno.writeFileSync(filePath, new Uint8Array());
      return;
    }
    throw err;
  }
}
