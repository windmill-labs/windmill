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
});
