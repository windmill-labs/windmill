import { SchemaProperty } from "./common.ts";

export interface ScriptMetadata {
  summary: string;
  description: string;
  lock: string | string[];
  is_template?: boolean;
  kind: string;
  schema: {
    $schema: string;
    type: string;
    properties: { [name: string]: SchemaProperty };
    required: string[];
  };
}

export function defaultScriptMetadata(): ScriptMetadata {
  return {
    summary: "",
    description: "",
    lock: "",
    kind: "script",
    schema: {
      $schema: "https://json-schema.org/draft/2020-12/schema",
      type: "object",
      properties: {},
      required: [],
    },
  };
}

export const scriptBootstrapCode = {
  python3: `def main():
    return "Hello world"
`,

  nativets: `export async function main() {
    return "Hello world";
}
`,

  bun: `// there are multiple modes to add as header: //nobundling //native //npm //nodejs
// https://www.windmill.dev/docs/getting_started/scripts_quickstart/typescript#modes

// import { toWords } from "number-to-words@1"
import * as wmill from "windmill-client"

// fill the type, or use the +Resource type to get a type-safe reference to a resource
// type Postgresql = object

export async function main(
  a: number,
  b: "my" | "enum",
  //c: Postgresql,
  //d: wmill.S3Object, // https://www.windmill.dev/docs/core_concepts/persistent_storage/large_data_files
  //d: DynSelect_foo, // https://www.windmill.dev/docs/core_concepts/json_schema_and_parsing#dynamic-select
  e = "inferred type string from default arg",
  f = { nested: "object" },
  g: {
    label: "Variant 1",
    foo: string
  } | {
    label: "Variant 2",
    bar: number
  }
) {
  // let x = await wmill.getVariable('u/user/foo')
  return { foo: a };
}
`,

  deno: `export async function main() {
  return "Hello world";
}
`,

  go: `package inner
func main() (interface{}, error) {
    return "Hello world", nil
}
`,

  mysql: `SELECT 'Hello world' AS message
`,

  bigquery: `SELECT 'Hello world' AS message
`,

  snowflake: `SELECT 'Hello world' AS message
`,

  mssql: `SELECT 'Hello world' AS message
`,

  graphql: `query() {
    demo() {}
}`,

  postgresql: `SELECT 'Hello world' AS message
`,

  bash: `echo "Hello world"
`,
  duckdb: `SELECT 'Hello world' AS message`,

  oracledb: `SELECT 'Hello world' AS message`,
  powershell: `Write-Output "Hello world"`,

  php: `<?php
function main() {
  return "Hello world";
}
`,

  csharp: `class Script
{
    public static void Main()
    {
        Console.WriteLine("Hello World");
    }
}
  `,
  nu: `
def main [] {
  print "Hello World"
}
  `,

  rust: `fn main() -> Result<(), String> {
  println!("Hello World");
  Ok(())
}
`,

  ansible: `---
inventory:
  - resource_type: ansible_inventory
---
- name: Echo
  hosts: 127.0.0.1
  connection: local

  tasks:
  - name: Print debug message
    debug:
      msg: "Hello, world!"
`,
  java: `
public class Main {
  public static void main() {
    System.out.println("Hello World");
  }
}
`,
  ruby: `
def main a, b, c
  puts a, b, c
end
`,
  // for related places search: ADD_NEW_LANG
};
