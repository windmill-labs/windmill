// Modified from: https://raw.githubusercontent.com/epoberezkin/fast-deep-equal/master/src/index.jst
// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-nocheck This file is copied from a JS project, so it's not type-safe.

import { log, encodeHex, SEP } from "./deps.ts";
import crypto from "node:crypto";

export function deepEqual<T>(a: T, b: T): boolean {
  if (a === b) return true;

  if (a && b && typeof a === "object" && typeof b === "object") {
    if (a.constructor !== b.constructor) return false;

    let length, i;
    if (Array.isArray(a)) {
      length = a.length;
      if (length != b.length) return false;
      for (i = length; i-- !== 0;) {
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
      for (i = length; i-- !== 0;) {
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

    for (i = length; i-- !== 0;) {
      if (!Object.prototype.hasOwnProperty.call(b, keys[i])) return false;
    }

    for (i = length; i-- !== 0;) {
      const key = keys[i];
      if (!deepEqual(a[key], b[key])) return false;
    }

    return true;
  }

  // true if both NaN, false otherwise
  return a !== a && b !== b;
}

export function getHeaders(): Record<string, string> | undefined {
  const headers = Deno.env.get("HEADERS");
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
  for await (const e of Deno.readDir(path)) {
    const npath = path + "/" + e.name;
    if (e.isFile) {
      hashes.push(await generateHashFromBuffer(await Deno.readFile(npath)));
    } else if (e.isDirectory && !e.isSymlink) {
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
  return encodeHex(hashBuffer);
}

// export async function readInlinePath(path: string): Promise<string> {
//   return await Deno.readTextFile(path.replaceAll("/", SEP));
// }

export function readInlinePathSync(path: string): string {
  return Deno.readTextFileSync(path.replaceAll("/", SEP));
}

export function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export function isFileResource(path: string): boolean {
  const splitPath = path.split(".");
  return (
    splitPath.length >= 4 &&
    splitPath[1] == "resource" &&
    splitPath[2] == "file"
  );
}

export function printSync(input: string | Uint8Array, to = Deno.stdout) {
  let bytesWritten = 0
  const bytes = typeof input === 'string' ? new TextEncoder().encode(input) : input
  while (bytesWritten < bytes.length) {
    bytesWritten += to.writeSync(bytes.subarray(bytesWritten))
  }
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
    const repoPath = repositories[0].git_repo_resource_path.replace(/^\$res:/, "");
    log.info(`Using repository: ${repoPath}`);
    return repositories[0];
  }

  // Check if we're in a non-interactive environment
  const isInteractive = Deno.stdin.isTerminal() && Deno.stdout.isTerminal();

  if (!isInteractive) {
    const repoPaths = repositories.map(r => r.git_repo_resource_path.replace(/^\$res:/, ""));
    throw new Error(`Multiple repositories found: ${repoPaths.join(', ')}. Use --repository to specify which one to ${operation || 'use'}.`);
  }

  // Import Select dynamically to avoid dependency issues
  const { Select } = await import("./deps.ts");

  console.log(`\nMultiple repositories found. Please select which repository to ${operation || 'use'}:\n`);

  const selectedRepo = await Select.prompt({
    message: `Select repository for ${operation || 'operation'}:`,
    options: repositories.map((repo, index) => {
      const displayPath = repo.git_repo_resource_path.replace(/^\$res:/, "");
      return {
        name: `${index + 1}. ${displayPath}`,
        value: repo.git_repo_resource_path
      };
    })
  });

  return repositories.find((r) => r.git_repo_resource_path === selectedRepo)!;
}
