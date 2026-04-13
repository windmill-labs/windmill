import { appendFile, mkdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { getAiEvalsRoot, getRepoRoot } from "./cases";
import type {
  BenchmarkArtifactFile,
  BenchmarkCaseResult,
  BenchmarkRunResult,
  BenchmarkTokenUsage,
  EvalMode,
} from "./types";

export async function writeRunResult(
  result: BenchmarkRunResult,
  outputPath?: string
): Promise<string> {
  const targetPath = resolveRunOutputPath(result.mode, outputPath);
  await mkdir(path.dirname(targetPath), { recursive: true });
  await writeFile(targetPath, JSON.stringify(toSerializableRunResult(result), null, 2) + "\n", "utf8");
  return targetPath;
}

export async function appendHistoryRecord(
  result: BenchmarkRunResult,
  historyPath = resolveHistoryPath(result.mode)
): Promise<string> {
  await mkdir(path.dirname(historyPath), { recursive: true });
  await appendFile(historyPath, JSON.stringify(toHistoryRecord(result)) + "\n", "utf8");
  return historyPath;
}

export async function writeRunArtifacts(
  result: BenchmarkRunResult,
  outputPath?: string
): Promise<string | null> {
  const targetPath = resolveRunOutputPath(result.mode, outputPath);
  const artifactRoot = defaultArtifactsRoot(targetPath);

  await rm(artifactRoot, { recursive: true, force: true });

  let wroteArtifacts = false;
  for (const caseResult of result.cases) {
    for (const attempt of caseResult.attempts) {
      const artifactFiles = attempt.artifactFiles ?? [];
      if (artifactFiles.length === 0) {
        attempt.artifactsPath = null;
        continue;
      }

      const attemptDir = path.join(artifactRoot, caseResult.id, `attempt-${attempt.attempt}`);
      await writeArtifactFiles(attemptDir, artifactFiles);
      attempt.artifactsPath = attemptDir;
      wroteArtifacts = true;
    }
  }

  result.artifactsPath = wroteArtifacts ? artifactRoot : null;
  return result.artifactsPath ?? null;
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
  const tokenUsageTotal = input.caseResults.reduce<BenchmarkTokenUsage | null>(
    (sum, entry) => {
      for (const attempt of entry.attempts) {
        if (!attempt.tokenUsage) {
          continue;
        }
        sum ??= { prompt: 0, completion: 0, total: 0 };
        sum.prompt += attempt.tokenUsage.prompt;
        sum.completion += attempt.tokenUsage.completion;
        sum.total += attempt.tokenUsage.total;
      }
      return sum;
    },
    null
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
    totalTokenUsage: tokenUsageTotal,
    averageTokenUsagePerAttempt:
      attemptCount === 0 || !tokenUsageTotal
        ? null
        : {
            prompt: tokenUsageTotal.prompt / attemptCount,
            completion: tokenUsageTotal.completion / attemptCount,
            total: tokenUsageTotal.total / attemptCount,
          },
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

export function resolveRunOutputPath(mode: EvalMode, outputPath?: string): string {
  return outputPath ?? path.join(getAiEvalsRoot(), "results", defaultFileName(mode));
}

export function resolveHistoryPath(mode: EvalMode): string {
  return path.join(getAiEvalsRoot(), "history", `${mode}.jsonl`);
}

function defaultArtifactsRoot(resultPath: string): string {
  return resultPath.endsWith(".json")
    ? resultPath.slice(0, -".json".length)
    : `${resultPath}.artifacts`;
}

async function writeArtifactFiles(
  rootDir: string,
  files: BenchmarkArtifactFile[]
): Promise<void> {
  for (const file of files) {
    const relativePath = normalizeArtifactPath(file.path);
    const targetPath = path.join(rootDir, relativePath);
    await mkdir(path.dirname(targetPath), { recursive: true });
    await writeFile(targetPath, file.content, "utf8");
  }
}

function normalizeArtifactPath(filePath: string): string {
  const normalized = filePath.replaceAll("\\", "/").replace(/^\/+/, "");
  const parts = normalized.split("/").filter(Boolean);
  if (parts.length === 0 || parts.some((part) => part === "." || part === "..")) {
    throw new Error(`Invalid artifact path: ${filePath}`);
  }
  return parts.join("/");
}

function toSerializableRunResult(result: BenchmarkRunResult): BenchmarkRunResult {
  return {
    ...result,
    cases: result.cases.map((caseResult) => ({
      ...caseResult,
      attempts: caseResult.attempts.map(({ artifactFiles, ...attempt }) => attempt),
    })),
  };
}

function toHistoryRecord(result: BenchmarkRunResult) {
  const judgeScores = result.cases.flatMap((caseResult) =>
    caseResult.attempts.flatMap((attempt) =>
      typeof attempt.judgeScore === "number" ? [attempt.judgeScore] : []
    )
  );

  return {
    createdAt: result.createdAt,
    gitSha: result.gitSha,
    mode: result.mode,
    runs: result.runs,
    runModel: result.runModel,
    judgeModel: result.judgeModel,
    caseCount: result.caseCount,
    attemptCount: result.attemptCount,
    passedAttempts: result.passedAttempts,
    passRate: result.passRate,
    averageDurationMs: result.averageDurationMs,
    averageJudgeScore:
      judgeScores.length === 0
        ? null
        : judgeScores.reduce((sum, score) => sum + score, 0) / judgeScores.length,
    averageTokenUsagePerAttempt: result.averageTokenUsagePerAttempt ?? null,
    failedCaseIds: Array.from(
      new Set(
        result.cases
          .filter((caseResult) => caseResult.attempts.some((attempt) => !attempt.passed))
          .map((caseResult) => caseResult.id)
      )
    ),
    cases: result.cases.map((caseResult) => {
      const attemptCount = caseResult.attempts.length;
      const passedAttempts = caseResult.attempts.filter((attempt) => attempt.passed).length;
      const totalDurationMs = caseResult.attempts.reduce(
        (sum, attempt) => sum + attempt.durationMs,
        0
      );
      const judgeScores = caseResult.attempts.flatMap((attempt) =>
        typeof attempt.judgeScore === "number" ? [attempt.judgeScore] : []
      );
      const totalTokenUsage = caseResult.attempts.reduce<BenchmarkTokenUsage | null>(
        (sum, attempt) => {
          if (!attempt.tokenUsage) {
            return sum;
          }
          sum ??= { prompt: 0, completion: 0, total: 0 };
          sum.prompt += attempt.tokenUsage.prompt;
          sum.completion += attempt.tokenUsage.completion;
          sum.total += attempt.tokenUsage.total;
          return sum;
        },
        null
      );

      return {
        id: caseResult.id,
        attemptCount,
        passedAttempts,
        passRate: attemptCount === 0 ? 0 : passedAttempts / attemptCount,
        averageDurationMs: attemptCount === 0 ? 0 : totalDurationMs / attemptCount,
        averageJudgeScore:
          judgeScores.length === 0
            ? null
            : judgeScores.reduce((sum, score) => sum + score, 0) / judgeScores.length,
        averageTokenUsagePerAttempt:
          attemptCount === 0 || !totalTokenUsage
            ? null
            : {
                prompt: totalTokenUsage.prompt / attemptCount,
                completion: totalTokenUsage.completion / attemptCount,
                total: totalTokenUsage.total / attemptCount,
              },
      };
    }),
  };
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
