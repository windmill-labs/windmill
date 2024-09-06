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
export declare class Base32EncoderStream extends TransformStream<Uint8Array, string> {
    constructor();
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
export declare class Base32DecoderStream extends TransformStream<string, Uint8Array> {
    constructor();
}
//# sourceMappingURL=base32_stream.d.ts.map