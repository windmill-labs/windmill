import { type SchemaType } from "./_schema.js";
export type { SchemaType };
/** Options for {@linkcode parse}. */
export interface ParseOptions {
    /**
     * Name of the schema to use.
     *
     * @default {"default"}
     */
    schema?: SchemaType;
    /**
     * If `true`, duplicate keys will overwrite previous values. Otherwise,
     * duplicate keys will throw a {@linkcode SyntaxError}.
     *
     * @default {false}
     */
    allowDuplicateKeys?: boolean;
    /**
     * If defined, a function to call on warning messages taking an
     * {@linkcode Error} as its only argument.
     */
    onWarning?(error: Error): void;
}
/**
 * Parse and return a YAML string as a parsed YAML document object.
 *
 * Note: This does not support functions. Untrusted data is safe to parse.
 *
 * @example Usage
 * ```ts
 * import { parse } from "@std/yaml/parse";
 * import { assertEquals } from "@std/assert";
 *
 * const data = parse(`
 * id: 1
 * name: Alice
 * `);
 *
 * assertEquals(data, { id: 1, name: "Alice" });
 * ```
 *
 * @throws {SyntaxError} Throws error on invalid YAML.
 * @param content YAML string to parse.
 * @param options Parsing options.
 * @returns Parsed document.
 */
export declare function parse(content: string, options?: ParseOptions): unknown;
/**
 * Same as {@linkcode parse}, but understands multi-document YAML sources, and
 * returns multiple parsed YAML document objects.
 *
 * @example Usage
 * ```ts
 * import { parseAll } from "@std/yaml/parse";
 * import { assertEquals } from "@std/assert";
 *
 * const data = parseAll(`
 * ---
 * id: 1
 * name: Alice
 * ---
 * id: 2
 * name: Bob
 * ---
 * id: 3
 * name: Eve
 * `);
 * assertEquals(data, [ { id: 1, name: "Alice" }, { id: 2, name: "Bob" }, { id: 3, name: "Eve" }]);
 * ```
 *
 * @param content YAML string to parse.
 * @param options Parsing options.
 * @returns Array of parsed documents.
 */
export declare function parseAll(content: string, options?: ParseOptions): unknown;
//# sourceMappingURL=parse.d.ts.map