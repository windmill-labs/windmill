import { Schema, SchemaProperty } from "../../bootstrap/common.ts";

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
        res += `  ${name}: ${rec(prop.properties ?? {})}`;
      } else if (prop.type == "array") {
        res += `  ${name}: ${prop?.items?.type ?? "any"}[]`;
      } else {
        let typ = prop?.type ?? "any";
        if (typ == "integer") {
          typ = "number";
        }
        res += `  ${name}: ${typ}`;
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
