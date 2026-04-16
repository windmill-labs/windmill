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
});
