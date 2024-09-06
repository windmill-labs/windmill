/**
 * Converts a Uint8Array stream into a base64-encoded stream.
 *
 * @experimental **UNSTABLE**: New API, yet to be vetted.
 *
 * @see {@link https://www.rfc-editor.org/rfc/rfc4648.html#section-4}
 *
 * @example Usage
 * ```ts
 * import { assertEquals } from "@std/assert";
 * import { encodeBase64 } from "@std/encoding/base64";
 * import { Base64EncoderStream } from "@std/encoding/base64-stream";
 * import { toText } from "@std/streams/to-text";
 *
 * const stream = ReadableStream.from(["Hello,", " world!"])
 *   .pipeThrough(new TextEncoderStream())
 *   .pipeThrough(new Base64EncoderStream());
 *
 * assertEquals(await toText(stream), encodeBase64(new TextEncoder().encode("Hello, world!")));
 * ```
 */
export declare class Base64EncoderStream extends TransformStream<Uint8Array, string> {
    constructor();
}
/**
 * Decodes a base64-encoded stream into a Uint8Array stream.
 *
 * @experimental **UNSTABLE**: New API, yet to be vetted.
 *
 * @see {@link https://www.rfc-editor.org/rfc/rfc4648.html#section-4}
 *
 * @example Usage
 * ```ts
 * import { assertEquals } from "@std/assert";
 * import { Base64DecoderStream } from "@std/encoding/base64-stream";
 * import { toText } from "@std/streams/to-text";
 *
 * const stream = ReadableStream.from(["SGVsbG8s", "IHdvcmxkIQ=="])
 *   .pipeThrough(new Base64DecoderStream())
 *   .pipeThrough(new TextDecoderStream());
 *
 * assertEquals(await toText(stream), "Hello, world!");
 * ```
 */
export declare class Base64DecoderStream extends TransformStream<string, Uint8Array> {
    constructor();
}
//# sourceMappingURL=base64_stream.d.ts.map