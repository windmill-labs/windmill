/**
 * Returns the media type associated with the file extension. Values are
 * normalized to lower case and matched irrespective of a leading `.`.
 *
 * When `extension` has no associated type, the function returns `undefined`.
 *
 * @example
 * ```ts
 * import { typeByExtension } from "https://deno.land/std@$STD_VERSION/media_types/type_by_extension.ts";
 *
 * typeByExtension("js"); // `application/json`
 * typeByExtension(".HTML"); // `text/html`
 * typeByExtension("foo"); // undefined
 * typeByExtension("file.json"); // undefined
 * ```
 */
export declare function typeByExtension(extension: string): string | undefined;
