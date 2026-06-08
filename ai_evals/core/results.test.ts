import { mkdtemp, readFile, rm } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { describe, expect, it } from "bun:test";
import {
  appendHistoryRecord,
  buildRunResult,
  formatRunSummary,
} from "./results";
import type { BenchmarkCaseResult } from "./types";

function caseResult(
  attempts: BenchmarkCaseResult["attempts"],
): BenchmarkCaseResult {
  return {
    id: "case-1",
    prompt: "Do the thing",
    attempts,
  };
}

describe("benchmark results", () => {
  it("keeps success cost metrics separate from failed attempts", () => {
    const result = buildRunResult({
      mode: "global",
      runs: 1,
      runModel: "model-under-test",
      judgeModel: "judge-model",
      caseResults: [
        caseResult([
          {
            attempt: 1,
            passed: true,
            durationMs: 1000,
            assistantMessageCount: 1,
            toolCallCount: 1,
            toolsUsed: ["edit_script"],
            skillsInvoked: [],
            checks: [{ name: "edited", passed: true }],
            judgeScore: 100,
            judgeSummary: "ok",
            error: null,
            tokenUsage: { prompt: 100, completion: 20, total: 120 },
          },
          {
            attempt: 2,
            passed: false,
            durationMs: 100,
            assistantMessageCount: 1,
            toolCallCount: 0,
            toolsUsed: [],
            skillsInvoked: [],
            checks: [{ name: "edited", passed: false }],
            judgeScore: 10,
            judgeSummary: "missed",
            error: "failed",
            tokenUsage: { prompt: 10, completion: 5, total: 15 },
          },
        ]),
      ],
    });

    expect(result.attemptCount).toBe(2);
    expect(result.passedAttempts).toBe(1);
    expect(result.passRate).toBe(0.5);
    expect(result.averageDurationMs).toBe(550);
    expect(result.averagePassedDurationMs).toBe(1000);
    expect(result.totalTokenUsage).toEqual({
      prompt: 110,
      completion: 25,
      total: 135,
    });
    expect(result.totalPassedTokenUsage).toEqual({
      prompt: 100,
      completion: 20,
      total: 120,
    });
    expect(result.averageTokenUsagePerAttempt).toEqual({
      prompt: 55,
      completion: 12.5,
      total: 67.5,
    });
    expect(result.averageTokenUsagePerPassedAttempt).toEqual({
      prompt: 100,
      completion: 20,
      total: 120,
    });

    const summary = formatRunSummary(result);
    expect(summary).toContain("Average duration (passed): 1000ms");
    expect(summary).toContain("Average tokens (passed): 120 total");
    expect(summary).toContain("Average duration (all attempts): 550ms");
  });

  it("reports passed averages as unavailable when no attempt passes", () => {
    const result = buildRunResult({
      mode: "global",
      runs: 1,
      runModel: "model-under-test",
      judgeModel: "judge-model",
      caseResults: [
        caseResult([
          {
            attempt: 1,
            passed: false,
            durationMs: 100,
            assistantMessageCount: 1,
            toolCallCount: 0,
            toolsUsed: [],
            skillsInvoked: [],
            checks: [{ name: "edited", passed: false }],
            judgeScore: 10,
            judgeSummary: "missed",
            error: "failed",
            tokenUsage: { prompt: 10, completion: 5, total: 15 },
          },
        ]),
      ],
    });

    expect(result.averagePassedDurationMs).toBeNull();
    expect(result.totalPassedTokenUsage).toBeNull();
    expect(result.averageTokenUsagePerPassedAttempt).toBeNull();
    expect(formatRunSummary(result)).toContain(
      "Average duration (passed): n/a",
    );
  });

  it("normalizes passed token averages by passed attempts", () => {
    const result = buildRunResult({
      mode: "global",
      runs: 1,
      runModel: "model-under-test",
      judgeModel: "judge-model",
      caseResults: [
        caseResult([
          {
            attempt: 1,
            passed: true,
            durationMs: 1000,
            assistantMessageCount: 1,
            toolCallCount: 1,
            toolsUsed: ["edit_script"],
            skillsInvoked: [],
            checks: [{ name: "edited", passed: true }],
            judgeScore: 100,
            judgeSummary: "ok",
            error: null,
            tokenUsage: { prompt: 100, completion: 20, total: 120 },
          },
          {
            attempt: 2,
            passed: true,
            durationMs: 1200,
            assistantMessageCount: 1,
            toolCallCount: 1,
            toolsUsed: ["edit_script"],
            skillsInvoked: [],
            checks: [{ name: "edited", passed: true }],
            judgeScore: 100,
            judgeSummary: "ok",
            error: null,
            tokenUsage: null,
          },
        ]),
      ],
    });

    expect(result.passedAttempts).toBe(2);
    expect(result.totalPassedTokenUsage).toEqual({
      prompt: 100,
      completion: 20,
      total: 120,
    });
    expect(result.averageTokenUsagePerPassedAttempt).toEqual({
      prompt: 50,
      completion: 10,
      total: 60,
    });
  });

  it("records passed-attempt metrics in history", async () => {
    const tempDir = await mkdtemp(join(tmpdir(), "windmill-ai-evals-"));
    try {
      const historyPath = join(tempDir, "history.jsonl");
      const result = buildRunResult({
        mode: "global",
        runs: 1,
        runModel: "model-under-test",
        judgeModel: "judge-model",
        caseResults: [
          caseResult([
            {
              attempt: 1,
              passed: true,
              durationMs: 1000,
              assistantMessageCount: 1,
              toolCallCount: 1,
              toolsUsed: ["edit_script"],
              skillsInvoked: [],
              checks: [{ name: "edited", passed: true }],
              judgeScore: 100,
              judgeSummary: "ok",
              error: null,
              tokenUsage: { prompt: 100, completion: 20, total: 120 },
            },
            {
              attempt: 2,
              passed: false,
              durationMs: 100,
              assistantMessageCount: 1,
              toolCallCount: 0,
              toolsUsed: [],
              skillsInvoked: [],
              checks: [{ name: "edited", passed: false }],
              judgeScore: 10,
              judgeSummary: "missed",
              error: "failed",
              tokenUsage: { prompt: 10, completion: 5, total: 15 },
            },
          ]),
        ],
      });

      await appendHistoryRecord(result, historyPath);
      const record = JSON.parse(await readFile(historyPath, "utf8"));

      expect(record.averageDurationMs).toBe(550);
      expect(record.averagePassedDurationMs).toBe(1000);
      expect(record.averageTokenUsagePerAttempt.total).toBe(67.5);
      expect(record.averageTokenUsagePerPassedAttempt.total).toBe(120);
      expect(record.cases[0].averageDurationMs).toBe(550);
      expect(record.cases[0].averagePassedDurationMs).toBe(1000);
      expect(record.cases[0].averageTokenUsagePerAttempt.total).toBe(67.5);
      expect(record.cases[0].averageTokenUsagePerPassedAttempt.total).toBe(
        120,
      );
    } finally {
      await rm(tempDir, { recursive: true, force: true });
    }
  });
});
