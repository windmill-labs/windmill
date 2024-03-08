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

  powershell: `Write-Output "Hello world"`,
};
