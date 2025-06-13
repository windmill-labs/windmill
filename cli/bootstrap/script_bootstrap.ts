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

  bun: `export async function main() {
  return "Hello world";
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
public class Main {
  public static void main() {
    System.out.println("Hello World");
  }
}
`,
// for related places search: ADD_NEW_LANG 
};
