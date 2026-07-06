import { expect, test } from "bun:test";

import { compileResourceTypeToTsType } from "../src/utils/resource_types.ts";
import type { Schema } from "../bootstrap/common.ts";

// =============================================================================
// Resource-type namespace generation (WIN-2132)
//
// `compileResourceTypeToTsType` renders a JSON Schema into the body of a
// TypeScript type used in the generated `rt.d.ts` (RT namespace). JSON Schema
// property names are unconstrained, so a name with a colon, hyphen, or space
// is legal in the schema but not a valid bare TS identifier. Emitting it raw
// produced syntactically invalid output that broke `tsc`. These tests pin that
// such names are quoted while plain identifiers stay bare.
// =============================================================================

function schema(properties: Schema["properties"]): Schema {
  return {
    $schema: undefined,
    type: "object",
    properties,
    required: [],
  };
}

test("plain identifiers are emitted without quotes", () => {
  const out = compileResourceTypeToTsType(
    schema({
      host: { type: "string" },
      _port: { type: "integer" },
      $ref: { type: "boolean" },
    })
  );
  expect(out).toContain("  host: string");
  expect(out).toContain("  _port: number");
  expect(out).toContain("  $ref: boolean");
  expect(out).not.toContain('"host"');
});

test("non-identifier property names are double-quoted", () => {
  const out = compileResourceTypeToTsType(
    schema({
      "content-type": { type: "string" },
      "x:api:key": { type: "string" },
      "with space": { type: "integer" },
      "3leading": { type: "boolean" },
    })
  );
  expect(out).toContain('  "content-type": string');
  expect(out).toContain('  "x:api:key": string');
  expect(out).toContain('  "with space": number');
  expect(out).toContain('  "3leading": boolean');
});

test("nested object and array property names are quoted too", () => {
  const out = compileResourceTypeToTsType(
    schema({
      "nested-obj": {
        type: "object",
        properties: { "inner-key": { type: "string" } },
      },
      "arr-field": { type: "array", items: { type: "string" } },
    })
  );
  expect(out).toContain('"nested-obj": {');
  expect(out).toContain('"inner-key": string');
  expect(out).toContain('"arr-field": string[]');
});
