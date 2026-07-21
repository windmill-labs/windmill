import { describe, expect, it } from "bun:test";
import {
  validateAppState,
  validateCliWorkspace,
  validateGlobalState,
  validateScriptState,
  validateToolExpectations,
} from "./validators";

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

describe("validateToolExpectations", () => {
  it("accepts Windmill-prefixed schedule paths", () => {
    const checks = validateToolExpectations({
      run: {
        success: true,
        actual: {},
        assistantMessageCount: 1,
        toolCallCount: 1,
        toolsUsed: ["create_schedule"],
        toolCallDetails: [
          {
            name: "create_schedule",
            arguments: {
              path: "f/evals/greet_user_daily",
            },
          },
        ],
        skillsInvoked: [],
      },
      toolExpect: {
        requiredToolsUsed: ["create_schedule"],
        toolCallArgs: [
          {
            tool: "create_schedule",
            field: "path",
            stringStartsWithAnyOf: ["f/", "u/"],
            stringMustNotStartWithAnyOf: ["schedules/"],
          },
        ],
      },
    });

    expect(checks.every((check) => check.passed)).toBe(true);
  });

  it("rejects schedule-prefixed tool paths", () => {
    const checks = validateToolExpectations({
      run: {
        success: true,
        actual: {},
        assistantMessageCount: 1,
        toolCallCount: 1,
        toolsUsed: ["create_schedule"],
        toolCallDetails: [
          {
            name: "create_schedule",
            arguments: {
              path: "schedules/greet_user_daily",
            },
          },
        ],
        skillsInvoked: [],
      },
      toolExpect: {
        requiredToolsUsed: ["create_schedule"],
        toolCallArgs: [
          {
            tool: "create_schedule",
            field: "path",
            stringStartsWithAnyOf: ["f/", "u/"],
            stringMustNotStartWithAnyOf: ["schedules/"],
          },
        ],
      },
    });

    expect(checks).toContainEqual({
      name: "create_schedule.path uses an accepted prefix",
      passed: false,
      details: 'accepted prefixes: f/, u/; values: "schedules/greet_user_daily"',
    });
    expect(checks).toContainEqual({
      name: "create_schedule.path avoids rejected prefixes",
      passed: false,
      details: 'rejected prefixes: schedules/; values: "schedules/greet_user_daily"',
    });
  });

  it("rejects forbidden tool usage", () => {
    const checks = validateToolExpectations({
      run: {
        success: true,
        actual: {},
        assistantMessageCount: 1,
        toolCallCount: 1,
        toolsUsed: ["write_script", "deploy_workspace_item"],
        skillsInvoked: [],
      },
      toolExpect: {
        forbiddenToolsUsed: ["deploy_workspace_item"],
      },
    });

    expect(checks).toContainEqual({
      name: "does not use deploy_workspace_item",
      passed: false,
      details: "tools used: write_script, deploy_workspace_item",
    });
  });

  it("accepts a stringIncludesAnyOf substring regardless of case or position", () => {
    const checks = validateToolExpectations({
      run: {
        success: true,
        actual: {},
        assistantMessageCount: 1,
        toolCallCount: 1,
        toolsUsed: ["exec_datatable_sql"],
        toolCallDetails: [
          {
            name: "exec_datatable_sql",
            arguments: {
              sql: "WITH recent AS (SELECT * FROM orders) SELECT count(*) FROM recent",
            },
          },
        ],
        skillsInvoked: [],
      },
      toolExpect: {
        requiredToolsUsed: ["exec_datatable_sql"],
        toolCallArgs: [
          {
            tool: "exec_datatable_sql",
            field: "sql",
            stringIncludesAnyOf: ["select"],
          },
        ],
      },
    });

    expect(checks.every((check) => check.passed)).toBe(true);
  });

  it("accepts a stringIncludesAnyOf substring inside an array-valued field", () => {
    const checks = validateToolExpectations({
      run: {
        success: true,
        actual: {},
        assistantMessageCount: 1,
        toolCallCount: 1,
        toolsUsed: ["open_page"],
        toolCallDetails: [
          {
            name: "open_page",
            arguments: {
              page: "compare",
              items: ["script:f/evals/global/compare_review_demo"],
            },
          },
        ],
        skillsInvoked: [],
      },
      toolExpect: {
        requiredToolsUsed: ["open_page"],
        toolCallArgs: [
          {
            tool: "open_page",
            field: "items",
            stringIncludesAnyOf: ["f/evals/global/compare_review_demo"],
          },
        ],
      },
    });

    expect(checks.every((check) => check.passed)).toBe(true);
  });

  it("accepts stringIncludesAnyOf when only one of several calls matches", () => {
    // Existential: a mutation mixed with verification SELECTs still passes.
    const checks = validateToolExpectations({
      run: {
        success: true,
        actual: {},
        assistantMessageCount: 1,
        toolCallCount: 2,
        toolsUsed: ["exec_datatable_sql"],
        toolCallDetails: [
          {
            name: "exec_datatable_sql",
            arguments: { sql: "UPDATE orders SET status = 'shipped' WHERE id = 2" },
          },
          {
            name: "exec_datatable_sql",
            arguments: { sql: "SELECT * FROM orders WHERE id = 2" },
          },
        ],
        skillsInvoked: [],
      },
      toolExpect: {
        toolCallArgs: [
          {
            tool: "exec_datatable_sql",
            field: "sql",
            stringIncludesAnyOf: ["insert into", "update"],
          },
        ],
      },
    });

    expect(checks.every((check) => check.passed)).toBe(true);
  });

  it("rejects stringIncludesAnyOf when no call matches any substring", () => {
    const checks = validateToolExpectations({
      run: {
        success: true,
        actual: {},
        assistantMessageCount: 1,
        toolCallCount: 1,
        toolsUsed: ["exec_datatable_sql"],
        toolCallDetails: [
          {
            name: "exec_datatable_sql",
            arguments: {
              sql: "DROP TABLE orders",
            },
          },
        ],
        skillsInvoked: [],
      },
      toolExpect: {
        toolCallArgs: [
          {
            tool: "exec_datatable_sql",
            field: "sql",
            stringIncludesAnyOf: ["insert into", "update"],
          },
        ],
      },
    });

    expect(checks).toContainEqual({
      name: "exec_datatable_sql.sql includes a required substring",
      passed: false,
      details:
        'accepted substrings: insert into, update; values: "DROP TABLE orders"',
    });
  });

  it("passes requiredToolsAnyOf when any alternative in the group is used", () => {
    const checks = validateToolExpectations({
      run: {
        success: true,
        actual: {},
        assistantMessageCount: 1,
        toolCallCount: 1,
        toolsUsed: ["search_app", "patch_app_file"],
        skillsInvoked: [],
      },
      toolExpect: {
        requiredToolsAnyOf: [["read_app_file", "search_app"]],
      },
    });

    expect(checks).toContainEqual({
      name: "uses one of read_app_file, search_app",
      passed: true,
    });
  });

  it("fails requiredToolsAnyOf when no alternative in the group is used", () => {
    const checks = validateToolExpectations({
      run: {
        success: true,
        actual: {},
        assistantMessageCount: 1,
        toolCallCount: 1,
        toolsUsed: ["patch_app_file"],
        skillsInvoked: [],
      },
      toolExpect: {
        requiredToolsAnyOf: [["read_app_file", "search_app"]],
      },
    });

    expect(checks).toContainEqual({
      name: "uses one of read_app_file, search_app",
      passed: false,
      details: "tools used: patch_app_file",
    });
  });
});

describe("validateGlobalState", () => {
  it("accepts a required script draft", () => {
    const checks = validateGlobalState({
      actual: {
        drafts: [
          {
            type: "script",
            path: "f/evals/global/greet_user",
            language: "bun",
            value:
              "export async function main(name: string) {\n  return `Hello, ${name}!`\n}\n",
            isDraft: true,
          },
        ],
      },
      validate: {
        draftCountExactly: 1,
        requiredDrafts: [
          {
            type: "script",
            path: "f/evals/global/greet_user",
            language: "bun",
            valueIncludes: ["Hello"],
          },
        ],
      },
    });

    expect(checks.every((check) => check.passed)).toBe(true);
  });

  it("fails when a required draft is missing", () => {
    const checks = validateGlobalState({
      actual: {
        drafts: [],
      },
      validate: {
        requiredDrafts: [
          {
            type: "script",
            path: "f/evals/global/greet_user",
          },
        ],
      },
    });

    expect(checks).toContainEqual({
      name: "global includes script draft f/evals/global/greet_user",
      passed: false,
      details: "drafts: none",
    });
  });

  it("accepts a required script draft without an exact path", () => {
    const checks = validateGlobalState({
      actual: {
        drafts: [
          {
            type: "script",
            path: "f/team_tools/friendly_greeting",
            language: "bun",
            summary: "Friendly greeting helper",
            value:
              "export async function main(name: string) {\n  return `Hello, ${name}!`\n}\n",
            isDraft: true,
          },
        ],
      },
      validate: {
        draftCountExactly: 1,
        requiredDrafts: [
          {
            type: "script",
            pathIncludes: ["greeting"],
            language: "bun",
            summaryIncludes: ["Friendly"],
            valueIncludes: ["Hello"],
          },
        ],
      },
    });

    expect(checks.every((check) => check.passed)).toBe(true);
  });

  it("reports flexible global draft path filters when no draft matches", () => {
    const checks = validateGlobalState({
      actual: {
        drafts: [
          {
            type: "script",
            path: "f/team_tools/friendly_greeting",
            language: "bun",
            value:
              "export async function main(name: string) {\n  return `Hello, ${name}!`\n}\n",
            isDraft: true,
          },
        ],
      },
      validate: {
        requiredDrafts: [
          {
            type: "script",
            pathIncludes: ["invoice"],
          },
        ],
      },
    });

    expect(checks).toContainEqual({
      name: "global includes script draft (path includes invoice)",
      passed: false,
      details: "drafts: script:f/team_tools/friendly_greeting",
    });
  });

  it("does not require a TypeScript entrypoint for non-TypeScript script drafts", () => {
    const checks = validateGlobalState({
      actual: {
        drafts: [
          {
            type: "script",
            path: "f/evals/global/greet_python",
            language: "python3",
            value: "def main(name: str):\n    return f'Hello, {name}!'\n",
            isDraft: true,
          },
        ],
      },
    });

    expect(checks.some((check) => check.name.includes("exports entrypoint"))).toBe(
      false
    );
    expect(checks.every((check) => check.passed)).toBe(true);
  });

  it("allows read-only global cases without draft expectations", () => {
    const checks = validateGlobalState({
      actual: {
        drafts: [],
      },
    });

    expect(
      checks.some(
        (check) => check.name === "global produced at least one draft"
      )
    ).toBe(false);
    expect(checks.every((check) => check.passed)).toBe(true);
  });

  it("matches expected global draft fixtures", () => {
    const checks = validateGlobalState({
      actual: {
        drafts: [
          {
            type: "script",
            path: "f/evals/global/greet_user",
            language: "bun",
            value:
              "export async function main(name: string) {\r\n  return `Hello, ${name}!`\r\n}\r\n",
            isDraft: true,
          },
        ],
      },
      expected: {
        drafts: [
          {
            type: "script",
            path: "f/evals/global/greet_user",
            language: "bun",
            value:
              "export async function main(name: string) {\n  return `Hello, ${name}!`\n}\n",
            isDraft: true,
          },
        ],
      },
    });

    expect(checks).toContainEqual({
      name: "global drafts match expected",
      passed: true,
    });
  });

  it("fails when expected global draft fixtures differ", () => {
    const checks = validateGlobalState({
      actual: {
        drafts: [
          {
            type: "script",
            path: "f/evals/global/greet_user",
            language: "bun",
            value:
              "export async function main(name: string) {\n  return `Hello, ${name}!`\n}\n",
            isDraft: true,
          },
        ],
      },
      expected: {
        drafts: [
          {
            type: "script",
            path: "f/evals/global/greet_user",
            language: "bun",
            value:
              "export async function main(name: string) {\n  return `Bonjour, ${name}!`\n}\n",
            isDraft: true,
          },
        ],
      },
    });

    const expectedMatchCheck = checks.find(
      (check) => check.name === "global drafts match expected"
    );
    expect(expectedMatchCheck?.passed).toBe(false);
    expect(expectedMatchCheck?.details).toContain(
      "script:f/evals/global/greet_user value differs"
    );
    expect(expectedMatchCheck?.details).toContain("Hello");
    expect(expectedMatchCheck?.details).toContain("Bonjour");
  });

  it("explains expected global draft metadata mismatches", () => {
    const checks = validateGlobalState({
      actual: {
        drafts: [
          {
            type: "script",
            path: "f/evals/global/greet_user",
            language: "bun",
            value:
              "export async function main(name: string) {\n  return `Hello, ${name}!`\n}\n",
            isDraft: true,
          },
        ],
      },
      expected: {
        drafts: [
          {
            type: "script",
            path: "f/evals/global/greet_user",
            language: "python3",
            value:
              "export async function main(name: string) {\n  return `Hello, ${name}!`\n}\n",
            isDraft: true,
          },
        ],
      },
    });

    const expectedMatchCheck = checks.find(
      (check) => check.name === "global drafts match expected"
    );
    expect(expectedMatchCheck?.passed).toBe(false);
    expect(expectedMatchCheck?.details).toContain(
      "script:f/evals/global/greet_user language differs"
    );
    expect(expectedMatchCheck?.details).toContain('actual="bun"');
    expect(expectedMatchCheck?.details).toContain('expected="python3"');
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

  it("can require an exact datatable table count", () => {
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
                notes: {},
                extra_notes: {},
              },
            },
          },
        ],
      },
      validate: {
        datatableTableCountExactly: 1,
      },
    });

    expect(checks).toContainEqual({
      name: "app includes exactly 1 datatable table",
      passed: false,
      details: "expected exactly 1, got 2",
    });
  });

  it("validates app datatable code, tool usage, and forbidden storage", () => {
    const checks = validateAppState({
      actual: {
        frontend: {
          "/index.tsx":
            "import { backend } from './wmill'\nexport default function App() { void backend.listNotes(); return <div /> }\n",
        },
        backend: {
          listNotes: {
            name: "List notes",
            type: "inline",
            inlineScript: {
              language: "bun",
              content:
                "import * as wmill from 'windmill-client'\nexport async function main() { const sql = wmill.datatable(); return await sql`SELECT * FROM notes`.fetch() }\n",
            },
          },
        },
        datatables: [
          {
            datatable_name: "main",
            schemas: {
              public: {
                notes: {},
              },
            },
          },
        ],
      },
      toolsUsed: ["list_datatables", "get_datatable_table_schema"],
      validate: {
        requiredFrontendFileContent: [
          {
            path: "/index.tsx",
            includes: ["backend.listNotes"],
          },
        ],
        requiredBackendRunnableContent: [
          {
            key: "listNotes",
            includes: ["wmill.datatable", "select", "notes"],
          },
        ],
        requiredToolsUsed: ["list_datatables", "get_datatable_table_schema"],
        forbiddenAppContent: ["localStorage", "sessionStorage"],
      },
    });

    expect(checks.every((check) => check.passed)).toBe(true);
  });

  it("fails app datatable code validation when required code or tools are missing", () => {
    const checks = validateAppState({
      actual: {
        frontend: {
          "/index.tsx": "export default function App() { localStorage.setItem('x', 'y'); return <div /> }\n",
        },
        backend: {
          listNotes: {
            name: "List notes",
            type: "inline",
            inlineScript: {
              language: "bun",
              content: "export async function main() { return [] }\n",
            },
          },
        },
        datatables: [],
      },
      toolsUsed: ["list_files"],
      validate: {
        requiredBackendRunnableContent: [
          {
            key: "listNotes",
            includes: ["wmill.datatable", "notes"],
          },
        ],
        requiredToolsUsed: ["list_datatables"],
        forbiddenAppContent: ["localStorage"],
      },
    });

    expect(checks).toContainEqual({
      name: "listNotes backend runnable includes required content",
      passed: false,
      details: "missing snippets: wmill.datatable, notes",
    });
    expect(checks).toContainEqual({
      name: "tool list_datatables was used",
      passed: false,
      details: "tools used: list_files",
    });
    expect(checks).toContainEqual({
      name: "app does not include forbidden content 'localStorage'",
      passed: false,
      details: "forbidden snippet: localStorage",
    });
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

  it("matches skills by exact name instead of substring", () => {
    const checks = validateCliWorkspace({
      actualFiles: {},
      assistantOutput: "No workspace changes needed.",
      trace: {
        toolsUsed: [{ tool: "Skill", input: { skill: "write-flow-helper" }, timestamp: 1 }],
        skillsInvoked: ["write-flow-helper"],
        assistantMessageCount: 1,
        bashCommands: [],
        proposedCommands: [],
        executedWmillCommands: [],
        wmillInvocations: [],
        firstMutationToolIndex: null,
      },
      cliExpect: {
        requiredSkills: ["write-flow"],
        forbiddenSkills: ["write-flow"],
      },
    });

    expect(checks).toContainEqual({
      name: "invokes skill write-flow",
      passed: false,
      details: "skills=write-flow-helper",
    });
    expect(checks).toContainEqual({
      name: "does not invoke skill write-flow",
      passed: true,
    });
  });

  it("accepts ordered proposed commands when they appear in one concatenated entry", () => {
    const checks = validateCliWorkspace({
      actualFiles: {},
      assistantOutput: "Run wmill generate-metadata and then wmill sync push.",
      trace: {
        toolsUsed: [{ tool: "Skill", input: { skill: "cli-commands" }, timestamp: 1 }],
        skillsInvoked: ["cli-commands"],
        assistantMessageCount: 1,
        bashCommands: [],
        proposedCommands: ["wmill generate-metadata and then wmill sync push"],
        executedWmillCommands: [],
        wmillInvocations: [],
        firstMutationToolIndex: null,
      },
      cliExpect: {
        orderedProposedCommands: ["wmill generate-metadata", "wmill sync push"],
      },
    });

    expect(checks).toContainEqual({
      name: "assistant proposes expected commands in order",
      passed: true,
    });
  });

  it("fails skill-before-mutation checks cleanly when no mutation happened", () => {
    const checks = validateCliWorkspace({
      actualFiles: {},
      assistantOutput: "Run `wmill sync pull` first.",
      trace: {
        toolsUsed: [{ tool: "Skill", input: { skill: "cli-commands" }, timestamp: 1 }],
        skillsInvoked: ["cli-commands"],
        assistantMessageCount: 1,
        bashCommands: [],
        proposedCommands: ["wmill sync pull"],
        executedWmillCommands: [],
        wmillInvocations: [],
        firstMutationToolIndex: null,
      },
      cliExpect: {
        requiredSkillsBeforeFirstMutation: ["cli-commands"],
      },
    });

    expect(checks).toContainEqual({
      name: "invokes skill cli-commands before first mutation",
      passed: false,
      details: "firstSkillIndex=0; firstMutationIndex=none",
    });
  });
});
