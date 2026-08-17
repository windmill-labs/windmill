import { Schema, SchemaProperty } from "../../bootstrap/common.ts";

function quotePropName(name: string): string {
  return /^[a-zA-Z_$][a-zA-Z0-9_$]*$/.test(name) ? name : JSON.stringify(name);
}

export function compileResourceTypeToTsType(schema: Schema) {
  function rec(x: { [name: string]: SchemaProperty }, root = false) {
    let res = "{\n";
    const entries = Object.entries(x);
    if (entries.length == 0) {
      return "any";
    }
    let i = 0;
    for (let [name, prop] of entries) {
      if (prop.type == "object") {
        res += `  ${quotePropName(name)}: ${rec(prop.properties ?? {})}`;
      } else if (prop.type == "array") {
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

  return rec(schema.properties, true);
}
