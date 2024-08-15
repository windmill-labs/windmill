// Modified from: https://raw.githubusercontent.com/epoberezkin/fast-deep-equal/master/src/index.jst
// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-nocheck This file is copied from a JS project, so it's not type-safe.

import { log, encodeHex, SEP } from "./deps.ts";

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
    if (a.toString !== Object.prototype.toString) {
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
