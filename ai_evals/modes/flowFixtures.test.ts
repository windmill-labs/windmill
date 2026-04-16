import { describe, expect, it } from "bun:test";
import { validateFlowState } from "../core/validators";
import { normalizeFlowInitialFixture } from "./flowFixtures";

describe("normalizeFlowInitialFixture", () => {
  it("preserves initial flow metadata for validation", () => {
    const initialFlow = {
      summary: "existing summary",
      value: {
        modules: [
          {
            id: "a",
            value: {
              type: "rawscript",
              language: "bun",
              content: "export async function main() { return 1; }",
              input_transforms: {},
            },
          },
        ],
      },
      schema: {
        $schema: "https://json-schema.org/draft/2020-12/schema",
        properties: {},
        required: [],
        type: "object",
      },
    };

    const initial = normalizeFlowInitialFixture(initialFlow);
    const checks = validateFlowState({
      actual: structuredClone(initialFlow),
      initial: initial.flowState,
    });

    const differsFromInitial = checks.find((check) => check.name === "flow differs from initial");
    expect(initial.flowState?.summary).toBe("existing summary");
    expect(differsFromInitial?.passed).toBe(false);
  });
});
