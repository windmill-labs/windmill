import { Schema, SchemaProperty } from "../../bootstrap/common.ts";

function quotePropName(name: string): string {
  return /^[a-zA-Z_$][a-zA-Z0-9_$]*$/.test(name) ? name : JSON.stringify(name);
}

function isPropertyMap(x: unknown): x is { [name: string]: SchemaProperty } {
  return typeof x === "object" && x !== null && !Array.isArray(x);
}

// Schemas are free-form jsonb: the column is nullable and hub types such as
// `record` or `dbt_profile` carry `{}` / `{"type":"object"}` with no
// `properties`. Anything that is not a property map compiles to `any`, since a
// throw here aborts the whole rt.d.ts generation.
export function compileResourceTypeToTsType(schema: Schema | undefined | null) {
  function rec(x: unknown): string {
    if (!isPropertyMap(x)) {
      return "any";
    }
    const entries = Object.entries(x);
    if (entries.length == 0) {
      return "any";
    }
    let res = "{\n";
    let i = 0;
    for (let [name, prop] of entries) {
      if (prop?.type == "object") {
        res += `  ${quotePropName(name)}: ${rec(prop.properties)}`;
      } else if (prop?.type == "array") {
        res += `  ${quotePropName(name)}: ${prop?.items?.type ?? "any"}[]`;
      } else {
        let typ = prop?.type ?? "any";
        if (typ == "integer") {
          typ = "number";
        }
        res += `  ${quotePropName(name)}: ${typ}`;
      }
      i++;
      if (i < entries.length) {
        res += ",\n";
      }
    }
    res += "\n}";
    return res;
  }

  return rec(schema?.properties);
}
