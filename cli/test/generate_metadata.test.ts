/**
 * Tests for WASM schema parsing across all supported languages.
 *
 * Calls `inferSchema` directly — no backend needed, fully local.
 * Verifies that each language's WASM parser loads correctly and produces
 * the expected JSON schema output.
 */

import { expect, test, describe } from "bun:test";
import { inferSchema } from "../src/utils/metadata.ts";
import type { ScriptLanguage } from "../src/utils/script_common.ts";

interface LanguageTestCase {
  language: ScriptLanguage;
  content: string;
  /** Property name to verify in schema.properties */
  expectedParam: string;
  /** Expected JSON schema type, or undefined to skip type check */
  expectedType?: string;
  /** If set, verify this resource format exists on the named param */
  expectedResourceParam?: { name: string; format: string };
}

const languageTestCases: LanguageTestCase[] = [
  {
    language: "python3",
    content: `def main(x: str):\n    return x\n`,
    expectedParam: "x",
    expectedType: "string",
  },
  {
    language: "bun",
    content: `export async function main(x: string) {\n  return x;\n}\n`,
    expectedParam: "x",
    expectedType: "string",
  },
  {
    language: "deno",
    content: `export async function main(x: string) {\n  return x;\n}\n`,
    expectedParam: "x",
    expectedType: "string",
  },
  {
    language: "nativets",
    content: `export async function main(x: string) {\n  return x;\n}\n`,
    expectedParam: "x",
    expectedType: "string",
  },
  {
    language: "go",
    content: `package inner\n\nfunc main(x string) (interface{}, error) {\n\treturn x, nil\n}\n`,
    expectedParam: "x",
    expectedType: "string",
  },
  {
    language: "bash",
    // Bash parser infers params from variable assignments like x="$1"
    content: `x="$1"\necho "$x"\n`,
    expectedParam: "x",
    expectedType: "string",
  },
  {
    language: "powershell",
    content: `param([string]$x)\nWrite-Output $x\n`,
    expectedParam: "x",
    expectedType: "string",
  },
  {
    language: "postgresql",
    content: `-- $1 name = default :: text\nSELECT $1::TEXT\n`,
    expectedParam: "name",
    expectedType: "string",
    expectedResourceParam: { name: "database", format: "resource-postgresql" },
  },
  {
    language: "mysql",
    // MySQL parser only auto-detects the database resource param
    content: `SELECT 1\n`,
    expectedParam: "database",
    expectedType: "object",
    expectedResourceParam: { name: "database", format: "resource-mysql" },
  },
  {
    language: "bigquery",
    content: `SELECT 1\n`,
    expectedParam: "database",
    expectedType: "object",
    expectedResourceParam: { name: "database", format: "resource-bigquery" },
  },
  {
    language: "snowflake",
    content: `SELECT 1\n`,
    expectedParam: "database",
    expectedType: "object",
    expectedResourceParam: { name: "database", format: "resource-snowflake" },
  },
  {
    language: "mssql",
    content: `SELECT 1\n`,
    expectedParam: "database",
    expectedType: "object",
    expectedResourceParam: {
      name: "database",
      format: "resource-ms_sql_server",
    },
  },
  {
    language: "oracledb",
    content: `SELECT 1 FROM dual\n`,
    expectedParam: "database",
    expectedType: "object",
    expectedResourceParam: { name: "database", format: "resource-oracledb" },
  },
  {
    language: "duckdb",
    // DuckDB parser doesn't auto-add a database resource
    content: `SELECT 1\n`,
    expectedParam: undefined as any,
    expectedType: undefined,
  },
  {
    language: "graphql",
    content: `query($name: String) {\n  user(name: $name) { id }\n}\n`,
    expectedParam: "name",
    expectedType: "string",
    expectedResourceParam: { name: "api", format: "resource-graphql" },
  },
  {
    language: "php",
    content: `<?php\nfunction main(string $x) {\n  return $x;\n}\n`,
    expectedParam: "x",
    expectedType: "string",
  },
  {
    language: "rust",
    content: `fn main(x: String) -> Result<String, String> {\n    Ok(x)\n}\n`,
    expectedParam: "x",
    expectedType: "string",
  },
  {
    language: "csharp",
    content: `class Script {\n    public static string Main(string x) {\n        return x;\n    }\n}\n`,
    expectedParam: "x",
    expectedType: "string",
  },
  {
    language: "nu",
    content: `def main [x: string] {\n  print $x\n}\n`,
    expectedParam: "x",
    expectedType: "string",
  },
  {
    language: "ansible",
    content: `---\ninventory:\n  - resource_type: ansible_inventory\n---\n- name: Test\n  hosts: 127.0.0.1\n  connection: local\n  tasks:\n  - name: Echo\n    debug:\n      msg: "hello"\n`,
    // Ansible parser produces "inventory.ini" as param name
    expectedParam: "inventory.ini",
    expectedType: undefined,
  },
  {
    language: "java",
    content: `public class Main {\n    public static String main(String x) {\n        return x;\n    }\n}\n`,
    expectedParam: "x",
    expectedType: "string",
  },
  {
    language: "ruby",
    content: `def main(x)\n  puts x\nend\n`,
    expectedParam: "x",
    expectedType: undefined, // Ruby is dynamically typed
  },
];

describe("generate-metadata schema parsing", () => {
  for (const tc of languageTestCases) {
    test(`${tc.language}: WASM parser loads and infers schema`, async () => {
      const result = await inferSchema(
        tc.language,
        tc.content,
        {},
        `test.${tc.language}`
      );

      expect(result).toBeDefined();
      expect(result.schema).toBeDefined();
      expect(result.schema.properties).toBeDefined();

      if (tc.expectedParam) {
        expect(result.schema.properties[tc.expectedParam]).toBeDefined();

        if (tc.expectedType !== undefined) {
          expect(result.schema.properties[tc.expectedParam].type).toEqual(
            tc.expectedType
          );
        }
      }

      if (tc.expectedResourceParam) {
        const rp = result.schema.properties[tc.expectedResourceParam.name];
        expect(rp).toBeDefined();
        expect(rp.type).toEqual("object");
        expect(rp.format).toEqual(tc.expectedResourceParam.format);
      }
    });
  }
});

const allLanguages: ScriptLanguage[] = [
  "python3", "bun", "deno", "nativets", "go", "bash", "powershell",
  "postgresql", "mysql", "bigquery", "snowflake", "mssql", "oracledb",
  "duckdb", "graphql", "php", "rust", "csharp", "nu", "ansible", "java", "ruby",
];

describe("generate-metadata invalid input handling", () => {
  for (const lang of allLanguages) {
    test(`${lang}: does not crash on invalid input`, async () => {
      const result = await inferSchema(
        lang,
        "THIS IS INVALID GARBAGE @#$%^&*()",
        {},
        `test.${lang}`
      );

      expect(result).toBeDefined();
      expect(result.schema).toBeDefined();
      expect(result.schema.properties).toBeDefined();
      // Should return a valid (possibly empty) schema, not throw
      expect(typeof result.schema.properties).toBe("object");
    });
  }
});
