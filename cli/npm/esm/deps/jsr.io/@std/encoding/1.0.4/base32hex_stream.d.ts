/**
 * Converts a Uint8Array stream into a base32hex-encoded stream.
 *
 * @experimental **UNSTABLE**: New API, yet to be vetted.
 *
 * @see {@link https://www.rfc-editor.org/rfc/rfc4648.html#section-6}
 *
 * @example Usage
 * ```ts
 * import { assertEquals } from "@std/assert";
 * import { encodeBase32Hex } from "@std/encoding/base32hex";
 * import { Base32HexEncoderStream } from "@std/encoding/base32hex-stream";
 * import { toText } from "@std/streams/to-text";
 *
 * const stream = ReadableStream.from(["Hello,", " world!"])
 *   .pipeThrough(new TextEncoderStream())
 *   .pipeThrough(new Base32HexEncoderStream());
 *
 * assertEquals(await toText(stream), encodeBase32Hex(new TextEncoder().encode("Hello, world!")));
 * ```
 */
export declare class Base32HexEncoderStream extends TransformStream<Uint8Array, string> {
    constructor();
}
/**
 * Decodes a base32hex-encoded stream into a Uint8Array stream.
 *
 * @experimental **UNSTABLE**: New API, yet to be vetted.
 *
 * @see {@link https://www.rfc-editor.org/rfc/rfc4648.html#section-6}
 *
 * @example Usage
 * ```ts
 * import { assertEquals } from "@std/assert";
 * import { Base32HexDecoderStream } from "@std/encoding/base32hex-stream";
 * import { toText } from "@std/streams/to-text";
 *
 * const stream = ReadableStream.from(["91IMOR3F5GG7ERRI", "DHI22==="])
 *   .pipeThrough(new Base32HexDecoderStream())
 *   .pipeThrough(new TextDecoderStream());
 *
 * assertEquals(await toText(stream), "Hello, world!");
 * ```
 */
export declare class Base32HexDecoderStream extends TransformStream<string, Uint8Array> {
    constructor();
}
//# sourceMappingURL=base32hex_stream.d.ts.map