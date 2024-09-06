// Ported from js-yaml v3.13.1:
// https://github.com/nodeca/js-yaml/commit/665aadda42349dcae869f12040d9b10ef18d12da
// Copyright 2011-2015 by Vitaly Puzrin. All rights reserved. MIT license.
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
// This module is browser compatible.

import type { KindType, Type } from "./_type.js";
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

/**
 * Name of the schema to use.
 *
 * > [!NOTE]
 * > It is recommended to use the schema that is most appropriate for your use
 * > case. Doing so will avoid any unnecessary processing and benefit
 * > performance.
 *
 * Options include:
 * - `failsafe`: supports generic mappings, generic sequences and generic
 * strings.
 * - `json`: extends `failsafe` schema by also supporting nulls, booleans,
 * integers and floats.
 * - `core`: functionally the same as `json` schema.
 * - `default`: extends `core` schema by also supporting binary, omap, pairs and
 * set types.
 * - `extended`: extends `default` schema by also supporting regular
 * expressions and undefined values.
 *
 * See
 * {@link https://yaml.org/spec/1.2.2/#chapter-10-recommended-schemas | YAML 1.2 spec}
 * for more details on the `failsafe`, `json` and `core` schemas.
 */
export type SchemaType = "failsafe" | "json" | "core" | "default" | "extended";

type ImplicitType = Type<"scalar">;
type ExplicitType = Type<KindType>;

export type TypeMap = Record<
  KindType | "fallback",
  Map<string, ExplicitType>
>;

function createTypeMap(
  implicitTypes: ImplicitType[],
  explicitTypes: ExplicitType[],
): TypeMap {
  const result: TypeMap = {
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

export interface Schema {
  implicitTypes: ImplicitType[];
  explicitTypes: ExplicitType[];
  typeMap: TypeMap;
}

function createSchema({ explicitTypes = [], implicitTypes = [], include }: {
  implicitTypes?: ImplicitType[];
  explicitTypes?: ExplicitType[];
  include?: Schema;
}): Schema {
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

export const SCHEMA_MAP = new Map<SchemaType, Schema>([
  ["core", CORE_SCHEMA],
  ["default", DEFAULT_SCHEMA],
  ["failsafe", FAILSAFE_SCHEMA],
  ["json", JSON_SCHEMA],
  ["extended", EXTENDED_SCHEMA],
]);
