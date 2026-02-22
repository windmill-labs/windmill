// Modified from: https://raw.githubusercontent.com/epoberezkin/fast-deep-equal/master/src/index.jst
// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-nocheck This file is copied from a JS project, so it's not type-safe.

import { colors } from "@cliffy/ansi/colors";
import * as log from "../core/log.ts";
import { sep as SEP } from "node:path";
import crypto from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
import { readdir, readFile } from "node:fs/promises";
import { fetchVersion } from "../core/context.ts";
import { updateGlobalVersions } from "../commands/sync/global.ts";
import { isRawAppPath } from "./resource_folders.ts";

export function deepEqual<T>(a: T, b: T): boolean {
  if (a === b) return true;

  if (a && b && typeof a === "object" && typeof b === "object") {
    if (a.constructor !== b.constructor) return false;

    let length, i;
    if (Array.isArray(a)) {
      length = a.length;
      if (length != b.length) return false;
      for (i = length; i-- !== 0; ) {
        if (!deepEqual(a[i], b[i])) return false;
      }
      return true;
    }

    if (a instanceof Map && b instanceof Map) {
      if (a.size !== b.size) return false;
      for (i of a.entries()) {
        if (!b.has(i[0])) return false;
      }
      for (i of a.entries()) {
        if (!deepEqual(i[1], b.get(i[0]))) return false;
      }
      return true;
    }

    if (a instanceof Set && b instanceof Set) {
      if (a.size !== b.size) return false;
      for (i of a.entries()) {
        if (!b.has(i[0])) return false;
      }
      return true;
    }

    if (ArrayBuffer.isView(a) && ArrayBuffer.isView(b)) {
      length = a.length;
      if (length != b.length) return false;
      for (i = length; i-- !== 0; ) {
        if (a[i] !== b[i]) return false;
      }
      return true;
    }

    if (a.constructor === RegExp) {
      return a.source === b.source && a.flags === b.flags;
    }
    if (a.valueOf !== Object.prototype.valueOf) {
      return a.valueOf() === b.valueOf();
    }
    if (
      a.toString !== Object.prototype.toString &&
      typeof a.toString == "function"
    ) {
      return a.toString() === b.toString();
    }

    const keys = Object.keys(a);
    length = keys.length;
    if (length !== Object.keys(b).length) return false;

    for (i = length; i-- !== 0; ) {
      if (!Object.prototype.hasOwnProperty.call(b, keys[i])) return false;
    }

    for (i = length; i-- !== 0; ) {
      const key = keys[i];
      if (!deepEqual(a[key], b[key])) return false;
    }

    return true;
  }

  // true if both NaN, false otherwise
  return a !== a && b !== b;
}

export function getHeaders(): Record<string, string> | undefined {
  const headers = process.env["HEADERS"];
  if (headers) {
    const parsedHeaders = Object.fromEntries(
      headers.split(",").map((h) => h.split(":").map((s) => s.trim()))
    );
    log.debug(
      "Headers from env keys: " + JSON.stringify(Object.keys(parsedHeaders))
    );
    return parsedHeaders;
  } else {
    return undefined;
  }
}

export async function digestDir(path: string, conf: string) {
  const hashes: string = [];
  const entries = await readdir(path, { withFileTypes: true });
  for (const e of entries) {
    const npath = path + "/" + e.name;
    if (e.isFile()) {
      hashes.push(await generateHashFromBuffer(await readFile(npath)));
    } else if (e.isDirectory() && !e.isSymbolicLink()) {
      hashes.push(await digestDir(npath, ""));
    }
  }
  return await generateHash(hashes.join("") + conf);
}

export async function generateHash(content: string): Promise<string> {
  const messageBuffer = new TextEncoder().encode(content);
  return await generateHashFromBuffer(messageBuffer);
}

export async function generateHashFromBuffer(
  content: BufferSource
): Promise<string> {
  const hashBuffer = await crypto.subtle.digest("SHA-256", content);
  return Buffer.from(hashBuffer).toString("hex");
}

export function readInlinePathSync(path: string): string {
  try {
    return readFileSync(path.replaceAll("/", SEP), "utf-8");
  } catch (error) {
    log.warn(`Error reading inline path: ${path}, ${error}`);
    return "";
  }
}

export function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export function isFileResource(path: string): boolean {
  const splitPath = path.split(".");

  // Check for pattern: *.resource.file.* (handles both base and branch-specific)
  return (
    splitPath.length >= 4 &&
    splitPath[splitPath.length - 3] == "resource" &&
    splitPath[splitPath.length - 2] == "file"
  );
}

export function isRawAppFile(path: string): boolean {
  return isRawAppPath(path);
}

export function isWorkspaceDependencies(path: string): boolean {
  return path.startsWith("dependencies/");
}

export function printSync(input: string | Uint8Array) {
  process.stdout.write(
    typeof input === "string" ? input : Buffer.from(input)
  );
}

// Repository interface for shared selection logic
export interface Repository {
  git_repo_resource_path: string;
}

// Shared repository selection logic
export async function selectRepository<T extends Repository>(
  repositories: T[],
  operation?: string
): Promise<T> {
  if (repositories.length === 0) {
    throw new Error("No git-sync repositories configured in workspace");
  }

  if (repositories.length === 1) {
    const repoPath = repositories[0].git_repo_resource_path.replace(
      /^\$res:/,
      ""
    );
    log.info(colors.cyan(`Auto-selected repository: ${colors.bold(repoPath)}`));
    return repositories[0];
  }

  // Check if we're in a non-interactive environment
  const isInteractive = !!process.stdin.isTTY && !!process.stdout.isTTY;

  if (!isInteractive) {
    const repoPaths = repositories.map((r) =>
      r.git_repo_resource_path.replace(/^\$res:/, "")
    );
    throw new Error(
      `Multiple repositories found: ${repoPaths.join(
        ", "
      )}. Use --repository to specify which one to ${operation || "use"}.`
    );
  }

  // Import Select dynamically to avoid dependency issues
  const { Select } = await import("@cliffy/prompt/select");

  console.log(
    `\nMultiple repositories found. Please select which repository to ${
      operation || "use"
    }:\n`
  );

  const selectedRepo = await Select.prompt({
    message: `Select repository for ${operation || "operation"}:`,
    options: repositories.map((repo, index) => {
      const displayPath = repo.git_repo_resource_path.replace(/^\$res:/, "");
      return {
        name: `${index + 1}. ${displayPath}`,
        value: repo.git_repo_resource_path,
      };
    }),
  });

  return repositories.find((r) => r.git_repo_resource_path === selectedRepo)!;
}

let isWin: boolean | undefined = undefined;
export async function getIsWin(): Promise<boolean> {
  if (isWin === undefined) {
    const os = await import("node:os");
    isWin = os.platform() === "win32";
  }
  return isWin;
}

/**
 * Writes content to a file only if it differs from existing content.
 * Creates parent directories if they don't exist.
 *
 * @param path - The file path to write to
 * @param content - The content to write
 * @returns true if file was written, false if skipped (content unchanged)
 */
export function writeIfChanged(path: string, content: string): boolean {
  try {
    const existing = readFileSync(path, "utf-8");
    if (existing === content) {
      return false; // Content unchanged, skip write
    }
  } catch (error: any) {
    // File doesn't exist or can't be read, proceed with write
    if (error?.code !== "ENOENT") {
      // If it's not a "not found" error, we might want to know about it
      // but still proceed with the write attempt
    }
  }

  writeFileSync(path, content, "utf-8");
  return true; // File was written
}

export async function fetchRemoteVersion(
  workspace: Workspace
): Promise<string> {
  const version = await fetchVersion(workspace.remote);
  if (version) {
    updateGlobalVersions(version);
  }
  log.info(colors.gray("Remote version: " + version));
}

export function toCamel(s: string) {
  return s.replace(/([-_][a-z])/gi, ($1) => {
    return $1.toUpperCase().replace("-", "").replace("_", "");
  });
}

export function capitalize(str: string): string {
  return str.charAt(0).toUpperCase() + str.slice(1);
}
