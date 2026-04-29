import { describe, expect, it } from "bun:test";
import { loadCases } from "./cases";

describe("loadCases", () => {
  it("loads backend preview runtime config for opt-in flow cases", async () => {
    const flowCases = await loadCases("flow");
    const caseEntry = flowCases.find((entry) => entry.id === "flow-test1-reuse-existing-script");

    expect(caseEntry?.runtime).toEqual({
      backendPreview: {
        args: {
          a: 2,
          b: 3,
        },
      },
    });
  });

  it("loads the workspace-flow preference benchmark case", async () => {
    const flowCases = await loadCases("flow");
    const caseEntry = flowCases.find(
      (entry) => entry.id === "flow-test13-prefer-existing-workspace-flow"
    );

    expect(caseEntry).toBeDefined();
    expect(caseEntry?.runtime).toEqual({
      backendPreview: {
        args: {
          a: 10,
          b: 5,
        },
      },
    });
    expect(caseEntry?.initialPath).toContain(
      "ai_evals/fixtures/frontend/flow/initial/test13_prefer_existing_workspace_flow_initial.json"
    );
    expect(caseEntry?.expectedPath).toContain(
      "ai_evals/fixtures/frontend/flow/expected/test13_prefer_existing_workspace_flow.json"
    );
  });

  it("loads app validation config for datatable-backed persistence cases", async () => {
    const appCases = await loadCases("app");
    const caseEntry = appCases.find(
      (entry) => entry.id === "app-test8-inventory-tracker-search-delete"
    );

    expect(caseEntry?.initialPath).toContain("ai_evals/fixtures/frontend/app/initial/inventory_tracker");
    expect(caseEntry?.validate).toEqual({
      requiredFrontendPaths: ["/index.tsx"],
      requiredBackendRunnableKeys: ["listInventory", "addInventory", "deleteInventory"],
      requiredBackendRunnableTypes: [
        { key: "listInventory", type: "inline" },
        { key: "addInventory", type: "inline" },
        { key: "deleteInventory", type: "inline" },
      ],
      requiredDatatables: [
        {
          datatableName: "main",
          schema: "public",
          table: "inventory_items",
        },
      ],
    });
  });

  it("loads the seeded recipe-book app modification case", async () => {
    const appCases = await loadCases("app");
    const caseEntry = appCases.find((entry) => entry.id === "app-test9-recipe-book-search-delete");

    expect(caseEntry?.initialPath).toContain("ai_evals/fixtures/frontend/app/initial/recipe_book");
    expect(caseEntry?.validate).toEqual({
      requiredFrontendPaths: ["/index.tsx"],
      requiredBackendRunnableKeys: ["listRecipes", "addRecipe", "deleteRecipe"],
      requiredBackendRunnableTypes: [
        { key: "listRecipes", type: "inline" },
        { key: "addRecipe", type: "inline" },
        { key: "deleteRecipe", type: "inline" },
      ],
      requiredDatatables: [
        {
          datatableName: "main",
          schema: "public",
          table: "recipes",
        },
      ],
    });
  });

  it("loads the file-manager rename save/cancel case", async () => {
    const appCases = await loadCases("app");
    const caseEntry = appCases.find(
      (entry) => entry.id === "app-test6-file-manager-rename-save-cancel"
    );

    expect(caseEntry?.initialPath).toContain("ai_evals/fixtures/frontend/app/initial/file_manager");
    expect(caseEntry?.validate).toMatchObject({
      requiredFrontendPaths: ["/index.tsx", "/components/FileItem.tsx"],
      requiredFrontendFileContent: [
        {
          path: "/components/FileItem.tsx",
          includes: ["Save", "Cancel", "Escape"],
        },
      ],
      forbiddenAppContent: ["onBlur={handleRename}"],
    });
  });

  it("loads the datatable-backed notes creation case", async () => {
    const appCases = await loadCases("app");
    const caseEntry = appCases.find((entry) => entry.id === "app-datatable-persistent-notes");

    expect(caseEntry?.initialPath).toContain("ai_evals/fixtures/frontend/app/initial/notes_datatable");
    expect(caseEntry?.runtime).toEqual({
      maxTurns: 10,
    });
    expect(caseEntry?.validate).toMatchObject({
      requiredFrontendPaths: ["/index.tsx"],
      requiredBackendRunnableKeys: ["listNotes", "addNote", "deleteNote"],
      datatableTableCountExactly: 1,
      requiredDatatables: [
        {
          datatableName: "main",
          schema: "public",
          table: "notes",
        },
      ],
      requiredToolsUsed: ["list_datatables", "get_datatable_table_schema"],
      forbiddenAppContent: ["localStorage", "sessionStorage", "indexedDB"],
    });
  });

  it("loads the session id micro-edit app case", async () => {
    const appCases = await loadCases("app");
    const caseEntry = appCases.find((entry) => entry.id === "app-test10-session-id-no-crypto");

    expect(caseEntry?.initialPath).toContain("ai_evals/fixtures/frontend/app/initial/session_id_chat");
    expect(caseEntry?.runtime).toEqual({
      maxTurns: 4,
    });
    expect(caseEntry?.validate).toEqual({
      requiredFrontendPaths: ["/index.tsx"],
      requiredBackendRunnableKeys: ["a"],
      requiredBackendRunnableTypes: [{ key: "a", type: "inline" }],
    });
  });

  it("loads app token usage cases with additional runtime context", async () => {
    const appCases = await loadCases("app");
    const datatableContextCase = appCases.find(
      (entry) => entry.id === "app-token-many-datatable-context"
    );

    expect(
      appCases.find((entry) => entry.id === "app-token-selected-large-frontend-context")
    ).toBeUndefined();
    expect(
      appCases.find((entry) => entry.id === "app-token-selected-large-backend-context")
    ).toBeUndefined();
    expect(datatableContextCase?.initialPath).toContain(
      "ai_evals/fixtures/frontend/app/initial/token_heavy_datatables"
    );
    expect(datatableContextCase?.runtime?.appContext?.additional).toHaveLength(10);
    expect(datatableContextCase?.runtime?.appContext?.additional?.[0]).toEqual({
      type: "datatable",
      datatableName: "main",
      schema: "analytics",
      table: "event_log_01",
    });
  });

  it("loads CLI behavior expectations for deploy-guidance cases", async () => {
    const cliCases = await loadCases("cli");
    const caseEntry = cliCases.find((entry) => entry.id === "bun-hello-script");

    expect(caseEntry?.cliExpect).toEqual({
      requiredSkills: ["write-script-bun"],
      requiredSkillsBeforeFirstMutation: ["write-script-bun"],
      forbiddenSkills: ["write-script-python3", "write-flow"],
      orderedAssistantMentions: ["wmill generate-metadata", "wmill sync push"],
      orderedProposedCommands: ["wmill generate-metadata", "wmill sync push"],
      forbiddenExecutedCommands: ["^wmill generate-metadata", "^wmill sync push"],
    });
  });

  it("loads tool expectations for workspace mutation cases", async () => {
    const scriptCases = await loadCases("script");
    const caseEntry = scriptCases.find(
      (entry) => entry.id === "script-test2-create-current-script-schedule"
    );

    expect(caseEntry?.toolExpect).toEqual({
      requiredToolsUsed: ["create_schedule"],
      toolCallArgs: [
        {
          tool: "create_schedule",
          field: "path",
          stringStartsWithAnyOf: ["f/", "u/"],
          stringMustNotStartWithAnyOf: ["schedules/"],
        },
      ],
    });
    expect(caseEntry?.skipJudge).toBe(true);
  });
});
