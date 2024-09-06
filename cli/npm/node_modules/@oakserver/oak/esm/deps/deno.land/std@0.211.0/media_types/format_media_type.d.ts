/** Serializes the media type and the optional parameters as a media type
 * conforming to RFC 2045 and RFC 2616.
 *
 * The type and parameter names are written in lower-case.
 *
 * When any of the arguments results in a standard violation then the return
 * value will be an empty string (`""`).
 *
 * @example
 * ```ts
 * import { formatMediaType } from "https://deno.land/std@$STD_VERSION/media_types/format_media_type.ts";
 *
 * formatMediaType("text/plain", { charset: "UTF-8" }); // `text/plain; charset=UTF-8`
 * ```
 */
export declare function formatMediaType(type: string, param?: Record<string, string> | Iterable<[string, string]>): string;
