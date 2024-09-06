/**
 * Converts a Uint8Array stream into a base64url-encoded stream.
 *
 * @experimental **UNSTABLE**: New API, yet to be vetted.
 *
 * @see {@link https://www.rfc-editor.org/rfc/rfc4648.html#section-5}
 *
 * @example Usage
 * ```ts
 * import { assertEquals } from "@std/assert";
 * import { encodeBase64Url } from "@std/encoding/base64url";
 * import { Base64UrlEncoderStream } from "@std/encoding/base64url-stream";
 * import { toText } from "@std/streams/to-text";
 *
 * const stream = ReadableStream.from(["Hello,", " world!"])
 *   .pipeThrough(new TextEncoderStream())
 *   .pipeThrough(new Base64UrlEncoderStream());
 *
 * assertEquals(await toText(stream), encodeBase64Url(new TextEncoder().encode("Hello, world!")));
 * ```
 */
export declare class Base64UrlEncoderStream extends TransformStream<Uint8Array, string> {
    constructor();
}
/**
 * Decodes a base64url-encoded stream into a Uint8Array stream.
 *
 * @experimental **UNSTABLE**: New API, yet to be vetted.
 *
 * @see {@link https://www.rfc-editor.org/rfc/rfc4648.html#section-5}
 *
 * @example Usage
 * ```ts
 * import { assertEquals } from "@std/assert";
 * import { encodeBase64Url } from "@std/encoding/base64url";
 * import { Base64UrlDecoderStream } from "@std/encoding/base64url-stream";
 * import { toText } from "@std/streams/to-text";
 *
 * const stream = ReadableStream.from(["SGVsbG8s", "IHdvcmxkIQ"])
 *   .pipeThrough(new Base64UrlDecoderStream())
 *   .pipeThrough(new TextDecoderStream());
 *
 * assertEquals(await toText(stream), "Hello, world!");
 * ```
 */
export declare class Base64UrlDecoderStream extends TransformStream<string, Uint8Array> {
    constructor();
}
//# sourceMappingURL=base64url_stream.d.ts.map