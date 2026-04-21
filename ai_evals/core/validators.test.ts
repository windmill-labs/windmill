import { describe, expect, it } from "bun:test";
import { validateAppState, validateCliWorkspace, validateScriptState } from "./validators";

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

describe("validateCliWorkspace", () => {
  it("accepts required CLI skills and proposed commands without execution", () => {
    const checks = validateCliWorkspace({
      actualFiles: {
        "f/evals/hello.ts": "export async function main(name: string) { return { greeting: `Hello, ${name}!` } }\n",
      },
      expectedFiles: {
        "f/evals/hello.ts": "export async function main(name: string)\nreturn { greeting: `Hello, ${name}!` }",
      },
      assistantOutput:
        "Created the script. Next run `wmill generate-metadata --yes` and then `wmill sync push`.",
      trace: {
        toolsUsed: [
          { tool: "Skill", input: { skill: "write-script-bun" }, timestamp: 1 },
          { tool: "Write", input: { file_path: "f/evals/hello.ts" }, timestamp: 2 },
        ],
        skillsInvoked: ["write-script-bun"],
        assistantMessageCount: 1,
        bashCommands: [],
        proposedCommands: ["wmill generate-metadata --yes", "wmill sync push"],
        executedWmillCommands: [],
        wmillInvocations: [],
        firstMutationToolIndex: 1,
      },
      cliExpect: {
        requiredSkills: ["write-script-bun"],
        requiredSkillsBeforeFirstMutation: ["write-script-bun"],
        orderedAssistantMentions: ["wmill generate-metadata", "wmill sync push"],
        orderedProposedCommands: ["wmill generate-metadata", "wmill sync push"],
        forbiddenExecutedCommands: ["^wmill generate-metadata", "^wmill sync push"],
      },
    });

    expect(checks.every((check) => check.passed)).toBe(true);
  });

  it("fails when a forbidden wmill command is executed", () => {
    const checks = validateCliWorkspace({
      actualFiles: {},
      assistantOutput: "Run `wmill sync push` when ready.",
      trace: {
        toolsUsed: [{ tool: "Bash", input: { command: "wmill sync push" }, timestamp: 1 }],
        skillsInvoked: [],
        assistantMessageCount: 1,
        bashCommands: ["wmill sync push"],
        proposedCommands: ["wmill sync push"],
        executedWmillCommands: ["wmill sync push"],
        wmillInvocations: [
          {
            argv: ["sync", "push"],
            cwd: "/tmp/workspace",
            timestamp: "2026-04-21T12:00:00+00:00",
          },
        ],
        firstMutationToolIndex: 0,
      },
      cliExpect: {
        forbiddenExecutedCommands: ["^wmill sync push"],
      },
    });

    expect(checks).toContainEqual({
      name: "does not execute ^wmill sync push",
      passed: false,
      details: "executed=wmill sync push",
    });
  });

  it("supports read-only guidance cases that must keep the workspace unchanged", () => {
    const checks = validateCliWorkspace({
      actualFiles: {},
      assistantOutput:
        "Use `wmill job get 123`, then `wmill job logs 123`, then `wmill job result 123`.",
      trace: {
        toolsUsed: [{ tool: "Skill", input: { skill: "cli-commands" }, timestamp: 1 }],
        skillsInvoked: ["cli-commands"],
        assistantMessageCount: 1,
        bashCommands: [],
        proposedCommands: ["wmill job get 123", "wmill job logs 123", "wmill job result 123"],
        executedWmillCommands: [],
        wmillInvocations: [],
        firstMutationToolIndex: null,
      },
      cliExpect: {
        requiredSkills: ["cli-commands"],
        workspaceUnchanged: true,
        orderedProposedCommands: [
          "wmill job get 123",
          "wmill job logs 123",
          "wmill job result 123",
        ],
        forbiddenProposedCommands: ["wmill sync push"],
      },
    });

    expect(checks.every((check) => check.passed)).toBe(true);
  });
});
