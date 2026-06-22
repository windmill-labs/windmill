import { appendFile, mkdir, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { getAiEvalsRoot, getRepoRoot } from "./cases";
import type {
  BenchmarkArtifactFile,
  BenchmarkAttemptResult,
  BenchmarkCaseResult,
  BenchmarkRunResult,
  BenchmarkTokenUsage,
  EvalMode,
} from "./types";

type AttemptAggregate = {
  attemptCount: number;
  durationTotal: number;
  tokenUsageAttemptCount: number;
  tokenUsageTotal: BenchmarkTokenUsage | null;
  finalContextAttemptCount: number;
  finalContextTotal: number;
  finalContextMax: number | null;
};

export async function writeRunResult(
  result: BenchmarkRunResult,
  outputPath?: string,
): Promise<string> {
  const targetPath = resolveRunOutputPath(result.mode, outputPath);
  await mkdir(path.dirname(targetPath), { recursive: true });
  await writeFile(
    targetPath,
    JSON.stringify(toSerializableRunResult(result), null, 2) + "\n",
    "utf8",
  );
  return targetPath;
}

export async function appendHistoryRecord(
  result: BenchmarkRunResult,
  historyPath = resolveHistoryPath(result.mode),
): Promise<string> {
  await mkdir(path.dirname(historyPath), { recursive: true });
  await appendFile(
    historyPath,
    JSON.stringify(toHistoryRecord(result)) + "\n",
    "utf8",
  );
  return historyPath;
}

export async function writeRunArtifacts(
  result: BenchmarkRunResult,
  outputPath?: string,
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

      const attemptDir = path.join(
        artifactRoot,
        caseResult.id,
        `attempt-${attempt.attempt}`,
      );
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
  const attempts = input.caseResults.flatMap((entry) => entry.attempts);
  const passedAttemptResults = attempts.filter((attempt) => attempt.passed);
  const attemptAggregate = aggregateAttempts(attempts);
  const passedAttemptAggregate = aggregateAttempts(passedAttemptResults);
  const attemptCount = attemptAggregate.attemptCount;
  const passedAttempts = passedAttemptAggregate.attemptCount;

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
    averageDurationMs:
      attemptCount === 0 ? 0 : attemptAggregate.durationTotal / attemptCount,
    averagePassedDurationMs: averageDuration(passedAttemptAggregate),
    totalTokenUsage: attemptAggregate.tokenUsageTotal,
    totalPassedTokenUsage: passedAttemptAggregate.tokenUsageTotal,
    averageTokenUsagePerAttempt:
      attemptCount === 0
        ? null
        : averageTokenUsage(attemptAggregate, attemptCount),
    averageTokenUsagePerPassedAttempt: averageTokenUsage(
      passedAttemptAggregate,
      passedAttempts,
    ),
    averageFinalContextTokensPassed: averageFinalContext(passedAttemptAggregate),
    maxFinalContextTokensPassed: passedAttemptAggregate.finalContextMax,
    cases: input.caseResults,
  };
}

export function formatRunSummary(result: BenchmarkRunResult): string {
  const lines = [
    `${result.mode} benchmark complete`,
    `Pass rate: ${formatPercent(result.passRate)} (${result.passedAttempts}/${result.attemptCount})`,
    `Average duration (passed): ${formatNullableDuration(result.averagePassedDurationMs ?? null)}`,
  ];

  if (result.averageTokenUsagePerPassedAttempt) {
    lines.push(
      `Average tokens (passed): ${formatTokenUsage(result.averageTokenUsagePerPassedAttempt)}`,
    );
  }
  if (result.averageFinalContextTokensPassed != null) {
    lines.push(
      `Final context size (passed): ${Math.round(result.averageFinalContextTokensPassed)} tokens (max ${Math.round(result.maxFinalContextTokensPassed ?? 0)})`,
    );
  }
  if (result.passedAttempts < result.attemptCount) {
    lines.push(
      `Average duration (all attempts): ${Math.round(result.averageDurationMs)}ms`,
    );
    if (result.averageTokenUsagePerAttempt) {
      lines.push(
        `Average tokens (all attempts): ${formatTokenUsage(result.averageTokenUsagePerAttempt)}`,
      );
    }
  }

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
      const failedChecks = attempt.checks
        .filter((check) => !check.passed)
        .map((check) => check.name);
      failures.push(
        `${caseResult.id} attempt ${attempt.attempt}: ${failedChecks.join(", ") || attempt.error || "failed"}`,
      );
    }
  }

  return failures;
}

function aggregateAttempts(attempts: BenchmarkAttemptResult[]): AttemptAggregate {
  const aggregate: AttemptAggregate = {
    attemptCount: attempts.length,
    durationTotal: 0,
    tokenUsageAttemptCount: 0,
    tokenUsageTotal: null,
    finalContextAttemptCount: 0,
    finalContextTotal: 0,
    finalContextMax: null,
  };

  for (const attempt of attempts) {
    aggregate.durationTotal += attempt.durationMs;
    if (typeof attempt.finalContextTokens === "number") {
      aggregate.finalContextAttemptCount += 1;
      aggregate.finalContextTotal += attempt.finalContextTokens;
      aggregate.finalContextMax = Math.max(
        aggregate.finalContextMax ?? 0,
        attempt.finalContextTokens,
      );
    }
    if (!attempt.tokenUsage) {
      continue;
    }
    aggregate.tokenUsageAttemptCount += 1;
    aggregate.tokenUsageTotal ??= { prompt: 0, completion: 0, total: 0 };
    aggregate.tokenUsageTotal.prompt += attempt.tokenUsage.prompt;
    aggregate.tokenUsageTotal.completion += attempt.tokenUsage.completion;
    aggregate.tokenUsageTotal.total += attempt.tokenUsage.total;
  }

  return aggregate;
}

function averageDuration(aggregate: AttemptAggregate): number | null {
  return aggregate.attemptCount === 0
    ? null
    : aggregate.durationTotal / aggregate.attemptCount;
}

function averageFinalContext(aggregate: AttemptAggregate): number | null {
  return aggregate.finalContextAttemptCount === 0
    ? null
    : aggregate.finalContextTotal / aggregate.finalContextAttemptCount;
}

function averageTokenUsage(
  aggregate: AttemptAggregate,
  denominator: number,
): BenchmarkTokenUsage | null {
  if (denominator === 0 || !aggregate.tokenUsageTotal) {
    return null;
  }
  return {
    prompt: aggregate.tokenUsageTotal.prompt / denominator,
    completion: aggregate.tokenUsageTotal.completion / denominator,
    total: aggregate.tokenUsageTotal.total / denominator,
  };
}

function formatNullableDuration(value: number | null): string {
  return value === null ? "n/a" : `${Math.round(value)}ms`;
}

function formatTokenUsage(value: BenchmarkTokenUsage): string {
  const total = Math.round(value.total);
  const prompt = Math.round(value.prompt);
  const completion = Math.round(value.completion);
  return `${total} total (${prompt} prompt, ${completion} completion)`;
}

function defaultFileName(mode: EvalMode): string {
  return `${new Date().toISOString().replaceAll(":", "-")}__${mode}.json`;
}

export function resolveRunOutputPath(
  mode: EvalMode,
  outputPath?: string,
): string {
  return (
    outputPath ?? path.join(getAiEvalsRoot(), "results", defaultFileName(mode))
  );
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
  files: BenchmarkArtifactFile[],
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
  if (
    parts.length === 0 ||
    parts.some((part) => part === "." || part === "..")
  ) {
    throw new Error(`Invalid artifact path: ${filePath}`);
  }
  return parts.join("/");
}

function toSerializableRunResult(
  result: BenchmarkRunResult,
): BenchmarkRunResult {
  return {
    ...result,
    cases: result.cases.map((caseResult) => ({
      ...caseResult,
      attempts: caseResult.attempts.map(
        ({ artifactFiles, ...attempt }) => attempt,
      ),
    })),
  };
}

function toHistoryRecord(result: BenchmarkRunResult) {
  const judgeScores = result.cases.flatMap((caseResult) =>
    caseResult.attempts.flatMap((attempt) =>
      typeof attempt.judgeScore === "number" ? [attempt.judgeScore] : [],
    ),
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
    averagePassedDurationMs: result.averagePassedDurationMs ?? null,
    averageJudgeScore:
      judgeScores.length === 0
        ? null
        : judgeScores.reduce((sum, score) => sum + score, 0) /
          judgeScores.length,
    averageTokenUsagePerAttempt: result.averageTokenUsagePerAttempt ?? null,
    averageTokenUsagePerPassedAttempt:
      result.averageTokenUsagePerPassedAttempt ?? null,
    averageFinalContextTokensPassed:
      result.averageFinalContextTokensPassed ?? null,
    maxFinalContextTokensPassed: result.maxFinalContextTokensPassed ?? null,
    failedCaseIds: Array.from(
      new Set(
        result.cases
          .filter((caseResult) =>
            caseResult.attempts.some((attempt) => !attempt.passed),
          )
          .map((caseResult) => caseResult.id),
      ),
    ),
    cases: result.cases.map((caseResult) => {
      const attemptAggregate = aggregateAttempts(caseResult.attempts);
      const passedAttemptAggregate = aggregateAttempts(
        caseResult.attempts.filter((attempt) => attempt.passed),
      );
      const attemptCount = attemptAggregate.attemptCount;
      const passedAttempts = passedAttemptAggregate.attemptCount;
      const judgeScores = caseResult.attempts.flatMap((attempt) =>
        typeof attempt.judgeScore === "number" ? [attempt.judgeScore] : [],
      );

      return {
        id: caseResult.id,
        attemptCount,
        passedAttempts,
        passRate: attemptCount === 0 ? 0 : passedAttempts / attemptCount,
        averageDurationMs:
          attemptCount === 0
            ? 0
            : attemptAggregate.durationTotal / attemptCount,
        averagePassedDurationMs: averageDuration(passedAttemptAggregate),
        averageJudgeScore:
          judgeScores.length === 0
            ? null
            : judgeScores.reduce((sum, score) => sum + score, 0) /
              judgeScores.length,
        averageTokenUsagePerAttempt:
          attemptCount === 0
            ? null
            : averageTokenUsage(attemptAggregate, attemptCount),
        averageTokenUsagePerPassedAttempt: averageTokenUsage(
          passedAttemptAggregate,
          passedAttempts,
        ),
        averageFinalContextTokensPassed: averageFinalContext(
          passedAttemptAggregate,
        ),
        maxFinalContextTokensPassed: passedAttemptAggregate.finalContextMax,
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
