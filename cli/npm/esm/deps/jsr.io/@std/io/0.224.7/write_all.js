// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
/**
 * Write all the content of the array buffer (`arr`) to the writer (`w`).
 *
 * @example Writing to stdout
 * ```ts no-assert
 * import { writeAll } from "@std/io/write-all";
 *
 * const contentBytes = new TextEncoder().encode("Hello World");
 * await writeAll(Deno.stdout, contentBytes);
 * ```
 *
 * @example Writing to file
 * ```ts no-eval no-assert
 * import { writeAll } from "@std/io/write-all";
 *
 * const contentBytes = new TextEncoder().encode("Hello World");
 * using file = await Deno.open('test.file', { write: true });
 * await writeAll(file, contentBytes);
 * ```
 *
 * @param writer The writer to write to
 * @param data The data to write
 */
export async function writeAll(writer, data) {
    let nwritten = 0;
    while (nwritten < data.length) {
        nwritten += await writer.write(data.subarray(nwritten));
    }
}
/**
 * Synchronously write all the content of the array buffer (`arr`) to the
 * writer (`w`).
 *
 * @example "riting to stdout
 * ```ts no-assert
 * import { writeAllSync } from "@std/io/write-all";
 *
 * const contentBytes = new TextEncoder().encode("Hello World");
 * writeAllSync(Deno.stdout, contentBytes);
 * ```
 *
 * @example Writing to file
 * ```ts no-eval no-assert
 * import { writeAllSync } from "@std/io/write-all";
 *
 * const contentBytes = new TextEncoder().encode("Hello World");
 * using file = Deno.openSync("test.file", { write: true });
 * writeAllSync(file, contentBytes);
 * ```
 *
 * @param writer The writer to write to
 * @param data The data to write
 */
export function writeAllSync(writer, data) {
    let nwritten = 0;
    while (nwritten < data.length) {
        nwritten += writer.writeSync(data.subarray(nwritten));
    }
}
