/**
 * For a given media type, return the most relevant extension, or `undefined`
 * if no extension can be found.
 *
 * Extensions are returned without a leading `.`.
 *
 * @example
 * ```ts
 * import { extension } from "https://deno.land/std@$STD_VERSION/media_types/extension.ts";
 *
 * extension("text/plain"); // `txt`
 * extension("application/json"); // `json`
 * extension("text/html; charset=UTF-8"); // `html`
 * extension("application/foo"); // undefined
 * ```
 */
export declare function extension(type: string): string | undefined;
