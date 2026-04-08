import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { getAiEvalsRoot, getRepoRoot } from "./cases";
import type {
  BenchmarkCaseResult,
  BenchmarkRunResult,
  EvalMode,
} from "./types";

export async function writeRunResult(
  result: BenchmarkRunResult,
  outputPath?: string
): Promise<string> {
  const targetPath =
    outputPath ?? path.join(getAiEvalsRoot(), "results", defaultFileName(result.mode));
  await mkdir(path.dirname(targetPath), { recursive: true });
  await writeFile(targetPath, JSON.stringify(result, null, 2) + "\n", "utf8");
  return targetPath;
}

export function buildRunResult(input: {
  mode: EvalMode;
  runs: number;
  runModel: string | null;
  judgeModel: string | null;
  caseResults: BenchmarkCaseResult[];
}): BenchmarkRunResult {
  const attemptCount = input.caseResults.reduce((sum, entry) => sum + entry.attempts.length, 0);
  const passedAttempts = input.caseResults.reduce(
    (sum, entry) => sum + entry.attempts.filter((attempt) => attempt.passed).length,
    0
  );
  const durationTotal = input.caseResults.reduce(
    (sum, entry) => sum + entry.attempts.reduce((inner, attempt) => inner + attempt.durationMs, 0),
    0
  );

  return {
    version: 1,
    mode: input.mode,
    createdAt: new Date().toISOString(),
    gitSha: getGitSha(),
    runs: input.runs,
    runModel: input.runModel,
    judgeModel: input.judgeModel,
    caseCount: input.caseResults.length,
    attemptCount,
    passedAttempts,
    passRate: attemptCount === 0 ? 0 : passedAttempts / attemptCount,
    averageDurationMs: attemptCount === 0 ? 0 : durationTotal / attemptCount,
    cases: input.caseResults,
  };
}

export function formatRunSummary(result: BenchmarkRunResult): string {
  const lines = [
    `${result.mode} benchmark complete`,
    `Pass rate: ${formatPercent(result.passRate)} (${result.passedAttempts}/${result.attemptCount})`,
    `Average duration: ${Math.round(result.averageDurationMs)}ms`,
  ];

  const failures = collectFailures(result);
  if (failures.length > 0) {
    lines.push("Failures:");
    for (const entry of failures.slice(0, 10)) {
      lines.push(`- ${entry}`);
    }
  }

  return lines.join("\n");
}

function collectFailures(result: BenchmarkRunResult): string[] {
  const failures: string[] = [];

  for (const caseResult of result.cases) {
    for (const attempt of caseResult.attempts) {
      if (attempt.passed) {
        continue;
      }
      const failedChecks = attempt.checks.filter((check) => !check.passed).map((check) => check.name);
      failures.push(
        `${caseResult.id} attempt ${attempt.attempt}: ${failedChecks.join(", ") || attempt.error || "failed"}`
      );
    }
  }

  return failures;
}

function defaultFileName(mode: EvalMode): string {
  return `${new Date().toISOString().replaceAll(":", "-")}__${mode}.json`;
}

function getGitSha(): string | null {
  try {
    return execFileSync("git", ["rev-parse", "HEAD"], {
      cwd: getRepoRoot(),
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
  } catch {
    return null;
  }
}

function formatPercent(value: number): string {
  return `${(value * 100).toFixed(1)}%`;
}
