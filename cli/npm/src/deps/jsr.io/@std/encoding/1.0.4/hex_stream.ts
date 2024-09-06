// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

/**
 * Utilities for encoding and decoding to and from hex in a streaming manner.
 *
 * @experimental **UNSTABLE**: New API, yet to be vetted.
 *
 * @module
 */

import { decodeHex, encodeHex } from "./hex.js";

/**
 * Converts a Uint8Array stream into a hex-encoded stream.
 *
 * @experimental **UNSTABLE**: New API, yet to be vetted.
 *
 * @see {@link https://www.rfc-editor.org/rfc/rfc4648.html#section-8}
 *
 * @example Usage
 * ```ts
 * import { assertEquals } from "@std/assert";
 * import { encodeHex } from "@std/encoding/hex";
 * import { HexEncoderStream } from "@std/encoding/hex-stream";
 * import { toText } from "@std/streams/to-text";
 *
 * const stream = ReadableStream.from(["Hello,", " world!"])
 *   .pipeThrough(new TextEncoderStream())
 *   .pipeThrough(new HexEncoderStream());
 *
 * assertEquals(await toText(stream), encodeHex(new TextEncoder().encode("Hello, world!")));
 * ```
 */
export class HexEncoderStream extends TransformStream<Uint8Array, string> {
  constructor() {
    super({
      transform(chunk, controller) {
        controller.enqueue(encodeHex(chunk));
      },
    });
  }
}

/**
 * Decodes a hex-encoded stream into a Uint8Array stream.
 *
 * @experimental **UNSTABLE**: New API, yet to be vetted.
 *
 * @see {@link https://www.rfc-editor.org/rfc/rfc4648.html#section-8}
 *
 * @example Usage
 * ```ts
 * import { assertEquals } from "@std/assert";
 * import { HexDecoderStream } from "@std/encoding/hex-stream";
 * import { toText } from "@std/streams/to-text";
 *
 * const stream = ReadableStream.from(["48656c6c6f2c", "20776f726c6421"])
 *   .pipeThrough(new HexDecoderStream())
 *   .pipeThrough(new TextDecoderStream());
 *
 * assertEquals(await toText(stream), "Hello, world!");
 * ```
 */
export class HexDecoderStream extends TransformStream<string, Uint8Array> {
  constructor() {
    let push = "";
    super({
      transform(chunk, controller) {
        push += chunk;
        if (push.length < 2) {
          return;
        }
        const remainder = -push.length % 2;
        controller.enqueue(decodeHex(push.slice(0, remainder || undefined)));
        push = remainder ? push.slice(remainder) : "";
      },
      flush(controller) {
        if (push.length) {
          controller.enqueue(decodeHex(push));
        }
      },
    });
  }
}
