// deno-lint-ignore no-explicit-any
import * as dntShim from "../../../../../../_dnt.shims.js";

const { Deno, process, Buffer } = dntShim.dntGlobalThis as any;
const { readSync: readSyncNode } = process
  ? await import("node:fs")
  : { readSync: null };
/**
 * Read from stdin.
 *
 * @internal
 * @param data Uint8Array to store the data.
 */
export function readSync(data: Uint8Array): number {
  if (Deno) {
    return Deno.stdout.readSync(data);
  } else if (readSyncNode) {
    const buffer = Buffer.alloc(data.byteLength);
    const bytesRead = readSyncNode(
      process.stdout.fd,
      buffer,
      0,
      buffer.length,
      null,
    );

    for (let i = 0; i < bytesRead; i++) {
      data[i] = buffer[i];
    }

    return bytesRead;
  } else {
    throw new Error("unsupported runtime");
  }
}
