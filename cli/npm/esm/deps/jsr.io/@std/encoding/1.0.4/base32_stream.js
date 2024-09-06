// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
/**
 * Utilities for encoding and decoding to and from base32 in a streaming manner.
 *
 * @experimental **UNSTABLE**: New API, yet to be vetted.
 *
 * @module
 */
import { decodeBase32, encodeBase32 } from "./base32.js";
/**
 * Converts a Uint8Array stream into a base32-encoded stream.
 *
 * @experimental **UNSTABLE**: New API, yet to be vetted.
 *
 * @see {@link https://www.rfc-editor.org/rfc/rfc4648.html#section-6}
 *
 * @example Usage
 * ```ts
 * import { assertEquals } from "@std/assert";
 * import { encodeBase32 } from "@std/encoding/base32";
 * import { Base32EncoderStream } from "@std/encoding/base32-stream";
 * import { toText } from "@std/streams/to-text";
 *
 * const stream = ReadableStream.from(["Hello,", " world!"])
 *   .pipeThrough(new TextEncoderStream())
 *   .pipeThrough(new Base32EncoderStream());
 *
 * assertEquals(await toText(stream), encodeBase32(new TextEncoder().encode("Hello, world!")));
 * ```
 */
export class Base32EncoderStream extends TransformStream {
    constructor() {
        let push = new Uint8Array(0);
        super({
            transform(chunk, controller) {
                const concat = new Uint8Array(push.length + chunk.length);
                concat.set(push);
                concat.set(chunk, push.length);
                const remainder = -concat.length % 5;
                controller.enqueue(encodeBase32(concat.slice(0, remainder || undefined)));
                push = remainder ? concat.slice(remainder) : new Uint8Array(0);
            },
            flush(controller) {
                if (push.length) {
                    controller.enqueue(encodeBase32(push));
                }
            },
        });
    }
}
/**
 * Decodes a base32-encoded stream into a Uint8Array stream.
 *
 * @experimental **UNSTABLE**: New API, yet to be vetted.
 *
 * @see {@link https://www.rfc-editor.org/rfc/rfc4648.html#section-6}
 *
 * @example Usage
 * ```ts
 * import { assertEquals } from "@std/assert";
 * import { Base32DecoderStream } from "@std/encoding/base32-stream";
 * import { toText } from "@std/streams/to-text";
 *
 * const stream = ReadableStream.from(["JBSWY3DPEBLW64TMMQQQ===="])
 *   .pipeThrough(new Base32DecoderStream())
 *   .pipeThrough(new TextDecoderStream());
 *
 * assertEquals(await toText(stream), "Hello World!");
 * ```
 */
export class Base32DecoderStream extends TransformStream {
    constructor() {
        let push = "";
        super({
            transform(chunk, controller) {
                push += chunk;
                if (push.length < 8) {
                    return;
                }
                const remainder = -push.length % 8;
                controller.enqueue(decodeBase32(push.slice(0, remainder || undefined)));
                push = remainder ? chunk.slice(remainder) : "";
            },
            flush(controller) {
                if (push.length) {
                    controller.enqueue(decodeBase32(push));
                }
            },
        });
    }
}
