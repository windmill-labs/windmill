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
export declare class HexEncoderStream extends TransformStream<Uint8Array, string> {
    constructor();
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
export declare class HexDecoderStream extends TransformStream<string, Uint8Array> {
    constructor();
}
//# sourceMappingURL=hex_stream.d.ts.map