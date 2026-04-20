import { describe, expect, it } from "bun:test";
import { validateAppState, validateScriptState } from "./validators";

describe("validateScriptState", () => {
  it("accepts semantically equivalent script implementations", () => {
    const checks = validateScriptState({
      actual: {
        path: "f/evals/greet_user.ts",
        lang: "bun",
        code: "export async function main(name: string): Promise<string> {\n  return `Hello, ${name}!`;\n}\n",
      },
      expected: {
        path: "f/evals/greet_user.ts",
        lang: "bun",
        code: "export async function main(name: string) {\n\treturn `Hello, ${name}!`\n}\n",
      },
    });

    expect(checks.every((check) => check.passed)).toBe(true);
  });

  it("still requires an exported main entrypoint", () => {
    const checks = validateScriptState({
      actual: {
        path: "f/evals/greet_user.ts",
        lang: "bun",
        code: "async function main(name: string) {\n  return `Hello, ${name}!`;\n}\n",
      },
    });

    expect(checks).toContainEqual({
      name: "script exports entrypoint",
      passed: false,
    });
  });
});

describe("validateAppState", () => {
  it("accepts app persistence requirements when a datatable table is registered", () => {
    const checks = validateAppState({
      actual: {
        frontend: {
          "/index.tsx": "import { backend } from 'wmill'\nexport default function App() { return <div /> }\n",
        },
        backend: {
          listRecipes: {
            name: "List recipes",
            type: "inline",
            inlineScript: {
              language: "bun",
              content:
                "import * as wmill from 'windmill-client'\nexport async function main() { const sql = wmill.datatable(); return await sql`select * from recipes`.fetch() }\n",
            },
          },
        },
        datatables: [
          {
            datatable_name: "main",
            schemas: {
              public: {
                recipes: {},
              },
            },
          },
        ],
      },
      validate: {
        datatableTableCountAtLeast: 1,
      },
    });

    expect(checks.every((check) => check.passed)).toBe(true);
  });

  it("fails app persistence requirements when no datatable table exists", () => {
    const checks = validateAppState({
      actual: {
        frontend: {
          "/index.tsx": "export default function App() { return <div /> }\n",
        },
        backend: {},
        datatables: [],
      },
      validate: {
        datatableTableCountAtLeast: 1,
      },
    });

    expect(checks).toContainEqual({
      name: "app includes at least 1 datatable table",
      passed: false,
      details: "expected at least 1, got 0",
    });
  });

  it("requires a specific datatable table when requested", () => {
    const checks = validateAppState({
      actual: {
        frontend: {
          "/index.tsx": "export default function App() { return <div /> }\n",
        },
        backend: {},
        datatables: [
          {
            datatable_name: "main",
            schemas: {
              public: {
                recipes: {},
              },
            },
          },
        ],
      },
      validate: {
        requiredDatatables: [
          {
            datatableName: "main",
            schema: "public",
            table: "recipes",
          },
        ],
      },
    });

    expect(checks.every((check) => check.passed)).toBe(true);
  });

  it("fails validation when frontend references a missing backend runnable", () => {
    const checks = validateAppState({
      actual: {
        frontend: {
          "/index.tsx":
            "import { backend } from 'wmill'\nexport default function App() { void backend.deleteRecipe({ id: 1 }); return <div /> }\n",
        },
        backend: {
          listRecipes: {
            name: "List recipes",
            type: "inline",
            inlineScript: {
              language: "bun",
              content: "export async function main() { return [] }\n",
            },
          },
        },
        datatables: [],
      },
    });

    expect(checks).toContainEqual({
      name: "frontend backend references resolve",
      passed: false,
      details: expect.stringContaining("deleteRecipe"),
    });
  });
});
