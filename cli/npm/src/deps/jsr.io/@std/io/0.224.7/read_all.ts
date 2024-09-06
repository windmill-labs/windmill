// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

import { concat } from "../../bytes/1.0.2/concat.js";
import { DEFAULT_CHUNK_SIZE } from "./_constants.js";
import type { Reader, ReaderSync } from "./types.js";

/**
 * Read {@linkcode Reader} `r` until EOF (`null`) and resolve to the content as
 * {@linkcode Uint8Array}.
 *
 * @example Usage
 * ```ts no-eval
 * import { readAll } from "@std/io/read-all";
 *
 * // Example from stdin
 * const stdinContent = await readAll(Deno.stdin);
 *
 * // Example from file
 * using file = await Deno.open("my_file.txt", {read: true});
 * const myFileContent = await readAll(file);
 * ```
 *
 * @param reader The reader to read from
 * @returns The content as Uint8Array
 */
export async function readAll(reader: Reader): Promise<Uint8Array> {
  const chunks: Uint8Array[] = [];
  while (true) {
    let chunk = new Uint8Array(DEFAULT_CHUNK_SIZE);
    const n = await reader.read(chunk);
    if (n === null) {
      break;
    }
    if (n < DEFAULT_CHUNK_SIZE) {
      chunk = chunk.subarray(0, n);
    }
    chunks.push(chunk);
  }
  return concat(chunks);
}

/**
 * Synchronously reads {@linkcode ReaderSync} `r` until EOF (`null`) and returns
 * the content as {@linkcode Uint8Array}.
 *
 * @example Usage
 * ```ts no-eval
 * import { readAllSync } from "@std/io/read-all";
 *
 * // Example from stdin
 * const stdinContent = readAllSync(Deno.stdin);
 *
 * // Example from file
 * using file = Deno.openSync("my_file.txt", {read: true});
 * const myFileContent = readAllSync(file);
 * ```
 *
 * @param reader The reader to read from
 * @returns The content as Uint8Array
 */
export function readAllSync(reader: ReaderSync): Uint8Array {
  const chunks: Uint8Array[] = [];
  while (true) {
    const chunk = new Uint8Array(DEFAULT_CHUNK_SIZE);
    const n = reader.readSync(chunk);
    if (n === null) {
      break;
    }
    if (n < DEFAULT_CHUNK_SIZE) {
      chunks.push(chunk.subarray(0, n));
      break;
    }
    chunks.push(chunk);
  }
  return concat(chunks);
}
