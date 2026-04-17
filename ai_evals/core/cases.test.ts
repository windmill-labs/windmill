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
});
