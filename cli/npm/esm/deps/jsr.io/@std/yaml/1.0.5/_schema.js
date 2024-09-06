// Ported from js-yaml v3.13.1:
// https://github.com/nodeca/js-yaml/commit/665aadda42349dcae869f12040d9b10ef18d12da
// Copyright 2011-2015 by Vitaly Puzrin. All rights reserved. MIT license.
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.
import { binary } from "./_type/binary.js";
import { bool } from "./_type/bool.js";
import { float } from "./_type/float.js";
import { int } from "./_type/int.js";
import { map } from "./_type/map.js";
import { merge } from "./_type/merge.js";
import { nil } from "./_type/nil.js";
import { omap } from "./_type/omap.js";
import { pairs } from "./_type/pairs.js";
import { regexp } from "./_type/regexp.js";
import { seq } from "./_type/seq.js";
import { set } from "./_type/set.js";
import { str } from "./_type/str.js";
import { timestamp } from "./_type/timestamp.js";
import { undefinedType } from "./_type/undefined.js";
function createTypeMap(implicitTypes, explicitTypes) {
    const result = {
        fallback: new Map(),
        mapping: new Map(),
        scalar: new Map(),
        sequence: new Map(),
    };
    const fallbackMap = result.fallback;
    for (const type of [...implicitTypes, ...explicitTypes]) {
        const map = result[type.kind];
        map.set(type.tag, type);
        fallbackMap.set(type.tag, type);
    }
    return result;
}
function createSchema({ explicitTypes = [], implicitTypes = [], include }) {
    if (include) {
        implicitTypes.push(...include.implicitTypes);
        explicitTypes.push(...include.explicitTypes);
    }
    const typeMap = createTypeMap(implicitTypes, explicitTypes);
    return { implicitTypes, explicitTypes, typeMap };
}
/**
 * Standard YAML's failsafe schema.
 *
 * @see {@link http://www.yaml.org/spec/1.2/spec.html#id2802346}
 */
const FAILSAFE_SCHEMA = createSchema({
    explicitTypes: [str, seq, map],
});
/**
 * Standard YAML's JSON schema.
 *
 * @see {@link http://www.yaml.org/spec/1.2/spec.html#id2803231}
 */
const JSON_SCHEMA = createSchema({
    implicitTypes: [nil, bool, int, float],
    include: FAILSAFE_SCHEMA,
});
/**
 * Standard YAML's core schema.
 *
 * @see {@link http://www.yaml.org/spec/1.2/spec.html#id2804923}
 */
const CORE_SCHEMA = createSchema({
    include: JSON_SCHEMA,
});
/**
 * Default YAML schema. It is not described in the YAML specification.
 */
export const DEFAULT_SCHEMA = createSchema({
    explicitTypes: [binary, omap, pairs, set],
    implicitTypes: [timestamp, merge],
    include: CORE_SCHEMA,
});
/***
 * Extends JS-YAML default schema with additional JavaScript types
 * It is not described in the YAML specification.
 * Functions are no longer supported for security reasons.
 *
 * @example
 * ```ts
 * import { parse } from "@std/yaml";
 *
 * const data = parse(
 *   `
 *   regexp:
 *     simple: !!js/regexp foobar
 *     modifiers: !!js/regexp /foobar/mi
 *   undefined: !!js/undefined ~
 * # Disabled, see: https://github.com/denoland/deno_std/pull/1275
 * #  function: !!js/function >
 * #    function foobar() {
 * #      return 'hello world!';
 * #    }
 * `,
 *   { schema: "extended" },
 * );
 * ```
 */
const EXTENDED_SCHEMA = createSchema({
    explicitTypes: [regexp, undefinedType],
    include: DEFAULT_SCHEMA,
});
export const SCHEMA_MAP = new Map([
    ["core", CORE_SCHEMA],
    ["default", DEFAULT_SCHEMA],
    ["failsafe", FAILSAFE_SCHEMA],
    ["json", JSON_SCHEMA],
    ["extended", EXTENDED_SCHEMA],
]);
