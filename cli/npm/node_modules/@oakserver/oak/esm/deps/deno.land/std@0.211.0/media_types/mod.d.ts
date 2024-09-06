/** Utility functions for media types (MIME types).
 *
 * This API is inspired by the GoLang [`mime`](https://pkg.go.dev/mime) package
 * and [jshttp/mime-types](https://github.com/jshttp/mime-types), and is
 * designed to integrate and improve the APIs from
 * [deno.land/x/media_types](https://deno.land/x/media_types).
 *
 * The `vendor` folder contains copy of the
 * [jshttp/mime-db](https://github.com/jshttp/mime-types) `db.json` file along
 * with its license.
 *
 * @module
 */
export * from "./content_type.js";
export * from "./extension.js";
export * from "./extensions_by_type.js";
export * from "./format_media_type.js";
export * from "./get_charset.js";
export * from "./parse_media_type.js";
export * from "./type_by_extension.js";
