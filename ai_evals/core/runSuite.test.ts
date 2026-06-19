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
});
