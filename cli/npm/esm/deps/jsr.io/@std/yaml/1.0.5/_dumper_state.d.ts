import { type Schema } from "./_schema.js";
import type { KindType, StyleVariant, Type } from "./_type.js";
export interface DumperStateOptions {
    /** indentation width to use (in spaces). */
    indent?: number;
    /** when true, adds an indentation level to array elements */
    arrayIndent?: boolean;
    /**
     * do not throw on invalid types (like function in the safe schema)
     * and skip pairs and single values with such types.
     */
    skipInvalid?: boolean;
    /**
     * specifies level of nesting, when to switch from
     * block to flow style for collections. -1 means block style everywhere
     */
    flowLevel?: number;
    /** Each tag may have own set of styles.	- "tag" => "style" map. */
    styles?: Record<string, StyleVariant>;
    /** specifies a schema to use. */
    schema?: Schema;
    /**
     * If true, sort keys when dumping YAML in ascending, ASCII character order.
     * If a function, use the function to sort the keys. (default: false)
     * If a function is specified, the function must return a negative value
     * if first argument is less than second argument, zero if they're equal
     * and a positive value otherwise.
     */
    sortKeys?: boolean | ((a: string, b: string) => number);
    /** set max line width. (default: 80) */
    lineWidth?: number;
    /**
     * if false, don't convert duplicate objects
     * into references (default: true)
     */
    useAnchors?: boolean;
    /**
     * if false don't try to be compatible with older yaml versions.
     * Currently: don't quote "yes", "no" and so on,
     * as required for YAML 1.1 (default: true)
     */
    compatMode?: boolean;
    /**
     * if true flow sequences will be condensed, omitting the
     * space between `key: value` or `a, b`. Eg. `'[a,b]'` or `{a:{b:c}}`.
     * Can be useful when using yaml for pretty URL query params
     * as spaces are %-encoded. (default: false).
     */
    condenseFlow?: boolean;
}
export declare class DumperState {
    indent: number;
    arrayIndent: boolean;
    skipInvalid: boolean;
    flowLevel: number;
    sortKeys: boolean | ((a: string, b: string) => number);
    lineWidth: number;
    useAnchors: boolean;
    compatMode: boolean;
    condenseFlow: boolean;
    implicitTypes: Type<"scalar">[];
    explicitTypes: Type<KindType>[];
    duplicates: unknown[];
    usedDuplicates: Set<unknown>;
    styleMap: Map<string, StyleVariant>;
    constructor({ schema, indent, arrayIndent, skipInvalid, flowLevel, styles, sortKeys, lineWidth, useAnchors, compatMode, condenseFlow, }: DumperStateOptions);
    stringifyScalar(string: string, { level, isKey }: {
        level: number;
        isKey: boolean;
    }): string;
    stringifyFlowSequence(array: unknown[], { level }: {
        level: number;
    }): string;
    stringifyBlockSequence(array: unknown[], { level, compact }: {
        level: number;
        compact: boolean;
    }): string;
    stringifyFlowMapping(object: Record<string, unknown>, { level }: {
        level: number;
    }): string;
    stringifyBlockMapping(object: Record<string, unknown>, { tag, level, compact }: {
        tag: string | null;
        level: number;
        compact: boolean;
    }): string;
    getTypeRepresentation(type: Type<KindType, unknown>, value: unknown): unknown;
    detectType(value: unknown): {
        tag: string | null;
        value: unknown;
    };
    stringifyNode(value: unknown, { level, block, compact, isKey }: {
        level: number;
        block: boolean;
        compact: boolean;
        isKey: boolean;
    }): string | null;
    stringify(value: unknown): string;
}
//# sourceMappingURL=_dumper_state.d.ts.map