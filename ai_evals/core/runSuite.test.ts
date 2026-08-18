import { describe, expect, it } from "bun:test";
import { runSuite } from "./runSuite";
import type { ModeRunner } from "./types";

const modeRunner: ModeRunner<undefined, undefined, { ok: boolean }> = {
  mode: "global",
  concurrency: 1,
  loadInitial: async () => undefined,
  loadExpected: async () => undefined,
  run: async () => ({
    success: true,
    actual: { ok: true },
    assistantMessageCount: 1,
    toolCallCount: 0,
    toolsUsed: [],
    skillsInvoked: [],
    tokenUsage: null,
  }),
  validate: () => [],
};

describe("runSuite", () => {
  it("skips judge checks when the run disables judge scoring", async () => {
    const [caseResult] = await runSuite({
      modeRunner,
      cases: [
        {
          id: "case-1",
          prompt: "Create a draft script",
          judgeChecklist: ["the output satisfies the prompt"],
        },
      ],
      runs: 1,
      runModel: "model-under-test",
      judgeModel: null,
    });

    const [attempt] = caseResult.attempts;
    expect(attempt.passed).toBe(true);
    expect(attempt.judgeScore).toBeNull();
    expect(attempt.judgeSummary).toBeNull();
    expect(attempt.checks.map((check) => check.name)).toEqual([
      "run succeeded",
    ]);
  });

  it("only requires run success when execution-only is enabled", async () => {
    let loadExpectedCalls = 0;
    let validateCalls = 0;
    let backendValidateCalls = 0;

    const executionOnlyRunner: ModeRunner<
      undefined,
      undefined,
      { ok: boolean }
    > = {
      ...modeRunner,
      loadExpected: async () => {
        loadExpectedCalls++;
        return undefined;
      },
      validate: () => {
        validateCalls++;
        return [{ name: "validator failed", passed: false }];
      },
      backendValidate: async () => {
        backendValidateCalls++;
        return {
          checks: [{ name: "backend validation failed", passed: false }],
        };
      },
    };

    const [caseResult] = await runSuite({
      modeRunner: executionOnlyRunner,
      cases: [
        {
          id: "case-1",
          prompt: "Create a draft script",
          expectedPath: "fixtures/expected.json",
          toolExpect: { requiredToolsUsed: ["write_script"] },
          judgeChecklist: ["the output satisfies the prompt"],
        },
      ],
      runs: 1,
      runModel: "model-under-test",
      judgeModel: "judge-model",
      executionOnly: true,
    });

    const [attempt] = caseResult.attempts;
    expect(attempt.passed).toBe(true);
    expect(attempt.judgeScore).toBeNull();
    expect(attempt.judgeSummary).toBeNull();
    expect(attempt.checks.map((check) => check.name)).toEqual([
      "run succeeded",
    ]);
    expect(loadExpectedCalls).toBe(0);
    expect(validateCalls).toBe(0);
    expect(backendValidateCalls).toBe(0);
  });
});
