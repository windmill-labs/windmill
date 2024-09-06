import type { db } from "./_db.js";
type DB = typeof db;
type ContentTypeToExtension = {
    [K in keyof DB]: DB[K] extends {
        "extensions": readonly string[];
    } ? DB[K]["extensions"][number] : never;
};
type KnownExtensionOrType = keyof ContentTypeToExtension | ContentTypeToExtension[keyof ContentTypeToExtension] | `.${ContentTypeToExtension[keyof ContentTypeToExtension]}`;
/**
 * Given an extension or media type, return a full `Content-Type` or
 * `Content-Disposition` header value.
 *
 * The function will treat the `extensionOrType` as a media type when it
 * contains a `/`, otherwise it will process it as an extension, with or without
 * the leading `.`.
 *
 * Returns `undefined` if unable to resolve the media type.
 *
 * > Note: a side effect of `deno/x/media_types` was that you could pass a file
 * > name (e.g. `file.json`) and it would return the content type. This behavior
 * > is intentionally not supported here. If you want to get an extension for a
 * > file name, use `extname()` from `std/path/mod.ts` to determine the
 * > extension and pass it here.
 *
 * @example
 * ```ts
 * import { contentType } from "https://deno.land/std@$STD_VERSION/media_types/content_type.ts";
 *
 * contentType(".json"); // `application/json; charset=UTF-8`
 * contentType("text/html"); // `text/html; charset=UTF-8`
 * contentType("text/html; charset=UTF-8"); // `text/html; charset=UTF-8`
 * contentType("txt"); // `text/plain; charset=UTF-8`
 * contentType("foo"); // undefined
 * contentType("file.json"); // undefined
 * ```
 */
export declare function contentType<T extends (string & {}) | KnownExtensionOrType>(extensionOrType: T): Lowercase<T> extends KnownExtensionOrType ? string : string | undefined;
export {};
