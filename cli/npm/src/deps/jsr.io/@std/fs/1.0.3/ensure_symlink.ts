// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
import * as dntShim from "../../../../../_dnt.shims.js";

import { dirname } from "../../path/1.0.4/dirname.js";
import { resolve } from "../../path/1.0.4/resolve.js";
import { ensureDir, ensureDirSync } from "./ensure_dir.js";
import { getFileInfoType, type PathType } from "./_get_file_info_type.js";
import { toPathString } from "./_to_path_string.js";

const isWindows = dntShim.Deno.build.os === "windows";

function resolveSymlinkTarget(target: string | URL, linkName: string | URL) {
  if (typeof target !== "string") return target; // URL is always absolute path
  if (typeof linkName === "string") {
    return resolve(dirname(linkName), target);
  } else {
    return new URL(target, linkName);
  }
}

function getSymlinkOption(
  type: PathType | undefined,
): dntShim.Deno.SymlinkOptions | undefined {
  return isWindows ? { type: type === "dir" ? "dir" : "file" } : undefined;
}

/**
 * Asynchronously ensures that the link exists, and points to a valid file.
 *
 * If the parent directories for the link do not exist, they are created. If the
 * link already exists, and it is not modified, this function does nothing. If
 * the link already exists, and it does not point to the given target, an error
 * is thrown.
 *
 * Requires `--allow-read` and `--allow-write` permissions.
 *
 * @see {@link https://docs.deno.com/runtime/manual/basics/permissions#file-system-access}
 * for more information on Deno's permissions system.
 *
 * @param target The source file path as a string or URL.
 * @param linkName The destination link path as a string or URL.
 *
 * @returns A void promise that resolves once the link exists.
 *
 * @example Usage
 * ```ts no-eval
 * import { ensureSymlink } from "@std/fs/ensure-symlink";
 *
 * await ensureSymlink("./folder/targetFile.dat", "./folder/targetFile.link.dat");
 * ```
 */
export async function ensureSymlink(
  target: string | URL,
  linkName: string | URL,
) {
  const targetRealPath = resolveSymlinkTarget(target, linkName);
  const srcStatInfo = await dntShim.Deno.lstat(targetRealPath);
  const srcFilePathType = getFileInfoType(srcStatInfo);

  await ensureDir(dirname(toPathString(linkName)));

  const options = getSymlinkOption(srcFilePathType);

  try {
    await dntShim.Deno.symlink(target, linkName, options);
  } catch (error) {
    if (!(error instanceof dntShim.Deno.errors.AlreadyExists)) {
      throw error;
    }
    const linkStatInfo = await dntShim.Deno.lstat(linkName);
    if (!linkStatInfo.isSymlink) {
      const type = getFileInfoType(linkStatInfo);
      throw new dntShim.Deno.errors.AlreadyExists(
        `A '${type}' already exists at the path: ${linkName}`,
      );
    }
    const linkPath = await dntShim.Deno.readLink(linkName);
    const linkRealPath = resolve(linkPath);
    if (linkRealPath !== targetRealPath) {
      throw new dntShim.Deno.errors.AlreadyExists(
        `A symlink targeting to an undesired path already exists: ${linkName} -> ${linkRealPath}`,
      );
    }
  }
}

/**
 * Synchronously ensures that the link exists, and points to a valid file.
 *
 * If the parent directories for the link do not exist, they are created. If the
 * link already exists, and it is not modified, this function does nothing. If
 * the link already exists, and it does not point to the given target, an error
 * is thrown.
 *
 * Requires `--allow-read` and `--allow-write` permissions.
 *
 * @see {@link https://docs.deno.com/runtime/manual/basics/permissions#file-system-access}
 * for more information on Deno's permissions system.
 *
 * @param target The source file path as a string or URL.
 * @param linkName The destination link path as a string or URL.
 * @returns A void value that returns once the link exists.
 *
 * @example Usage
 * ```ts no-eval
 * import { ensureSymlinkSync } from "@std/fs/ensure-symlink";
 *
 * ensureSymlinkSync("./folder/targetFile.dat", "./folder/targetFile.link.dat");
 * ```
 */
export function ensureSymlinkSync(
  target: string | URL,
  linkName: string | URL,
) {
  const targetRealPath = resolveSymlinkTarget(target, linkName);
  const srcStatInfo = dntShim.Deno.lstatSync(targetRealPath);
  const srcFilePathType = getFileInfoType(srcStatInfo);

  ensureDirSync(dirname(toPathString(linkName)));

  const options = getSymlinkOption(srcFilePathType);

  try {
    dntShim.Deno.symlinkSync(target, linkName, options);
  } catch (error) {
    if (!(error instanceof dntShim.Deno.errors.AlreadyExists)) {
      throw error;
    }
    const linkStatInfo = dntShim.Deno.lstatSync(linkName);
    if (!linkStatInfo.isSymlink) {
      const type = getFileInfoType(linkStatInfo);
      throw new dntShim.Deno.errors.AlreadyExists(
        `A '${type}' already exists at the path: ${linkName}`,
      );
    }
    const linkPath = dntShim.Deno.readLinkSync(linkName);
    const linkRealPath = resolve(linkPath);
    if (linkRealPath !== targetRealPath) {
      throw new dntShim.Deno.errors.AlreadyExists(
        `A symlink targeting to an undesired path already exists: ${linkName} -> ${linkRealPath}`,
      );
    }
  }
}
