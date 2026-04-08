import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export type SurfaceName = "cli" | "flow" | "app" | "script";
export type CanonicalSurfaceName =
  | "cli"
  | "frontend-flow"
  | "frontend-app"
  | "frontend-script";

export interface AttemptSummary {
  attempt: number;
  passed: boolean;
  durationMs: number;
  assistantMessageCount: number;
  toolCallCount: number;
  skillInvocationCount: number;
  skillsInvoked: string[];
  toolsUsed: string[];
  checks: Array<{ name: string; passed: boolean; required?: boolean }>;
  requiredFailedChecks: string[];
  expectedFiles: Array<{ path: string; exists: boolean }>;
  pathSignature: string;
  judgeScore: number | null;
  error: string | null;
}

export interface AggregateMetrics {
  totalRuns: number;
  passedRuns: number;
  passRate: number;
  averageDurationMs: number;
  medianDurationMs: number;
  latencyPerSuccessMs: number;
  averageAssistantMessages: number;
  averageToolCalls: number;
  averageSkillInvocations: number;
  distinctSkillsInvoked: string[];
  distinctToolsUsed: string[];
  requiredFailureCounts: Array<{ name: string; count: number }>;
  judgeScoreMean: number | null;
  judgeScoreMedian: number | null;
  judgeScoreP10: number | null;
}

export interface BenchmarkCaseResult extends AggregateMetrics {
  caseId: string;
  pathConsistency: number;
  attempts: AttemptSummary[];
}

export interface BenchmarkRunResult extends AggregateMetrics {
  version: "ai-evals-run-v1";
  createdAt: string;
  gitSha: string;
  label: string;
  surface: SurfaceName;
  canonicalSurface: CanonicalSurfaceName;
  provider: string;
  model: string;
  judgeModel: string | null;
  runs: number;
  totalCases: number;
  fullyPassedCases: number;
  caseIds: string[];
  config: Record<string, unknown>;
  caseResults: BenchmarkCaseResult[];
}

export interface FailureDelta {
  name: string;
  before: number;
  after: number;
  delta: number;
}

export interface CaseDelta {
  caseId: string;
  beforePassRate: number;
  afterPassRate: number;
  deltaPassRate: number;
  beforeFailures: string[];
  afterFailures: string[];
}

export interface BenchmarkRunDiff {
  surface: SurfaceName;
  beforeLabel: string;
  afterLabel: string;
  summary: {
    fullyPassedCasesDelta: number;
    passRateDelta: number;
    judgeScoreMeanDelta: number | null;
    averageDurationMsDelta: number;
  };
  improvedCases: CaseDelta[];
  regressedCases: CaseDelta[];
  failureDeltas: FailureDelta[];
}

const RESULTS_DIR = fileURLToPath(new URL("../results", import.meta.url));

export async function writeBenchmarkRunResult(
  result: BenchmarkRunResult,
  outputPath?: string
): Promise<string> {
  const resolvedPath = outputPath
    ? path.resolve(outputPath)
    : path.join(
        RESULTS_DIR,
        `${slugifyTimestamp(result.createdAt)}__${slugify(result.surface)}__${slugify(result.label)}__${result.gitSha.slice(0, 12)}.json`
      );

  await mkdir(path.dirname(resolvedPath), { recursive: true });
  await writeFile(resolvedPath, JSON.stringify(result, null, 2) + "\n", "utf8");
  return resolvedPath;
}

export async function readBenchmarkRunResult(
  inputPath: string
): Promise<BenchmarkRunResult> {
  const resolvedPath = path.resolve(inputPath);
  const raw = await readFile(resolvedPath, "utf8");
  const parsed = JSON.parse(raw) as BenchmarkRunResult;

  if (parsed.version !== "ai-evals-run-v1") {
    throw new Error(`Unsupported benchmark result version in ${resolvedPath}`);
  }

  if (!Array.isArray(parsed.caseResults) || parsed.caseResults.length === 0) {
    throw new Error(`Benchmark result ${resolvedPath} has no case results`);
  }

  return parsed;
}

export function buildBenchmarkRunDiff(
  before: BenchmarkRunResult,
  after: BenchmarkRunResult
): BenchmarkRunDiff {
  if (before.surface !== after.surface) {
    throw new Error(
      `Cannot diff ${before.surface} against ${after.surface}`
    );
  }

  const beforeCases = new Map(before.caseResults.map((entry) => [entry.caseId, entry]));
  const afterCases = new Map(after.caseResults.map((entry) => [entry.caseId, entry]));
  const sharedCaseIds = [...beforeCases.keys()].filter((caseId) => afterCases.has(caseId));

  const caseDeltas = sharedCaseIds.map((caseId) => {
    const beforeCase = beforeCases.get(caseId)!;
    const afterCase = afterCases.get(caseId)!;
    return {
      caseId,
      beforePassRate: beforeCase.passRate,
      afterPassRate: afterCase.passRate,
      deltaPassRate: afterCase.passRate - beforeCase.passRate,
      beforeFailures: beforeCase.requiredFailureCounts.map((entry) => entry.name),
      afterFailures: afterCase.requiredFailureCounts.map((entry) => entry.name)
    } satisfies CaseDelta;
  });

  const failureCounts = new Map<string, { before: number; after: number }>();
  for (const entry of before.requiredFailureCounts) {
    failureCounts.set(entry.name, { before: entry.count, after: 0 });
  }
  for (const entry of after.requiredFailureCounts) {
    const previous = failureCounts.get(entry.name) ?? { before: 0, after: 0 };
    failureCounts.set(entry.name, { ...previous, after: entry.count });
  }

  return {
    surface: before.surface,
    beforeLabel: before.label,
    afterLabel: after.label,
    summary: {
      fullyPassedCasesDelta: after.fullyPassedCases - before.fullyPassedCases,
      passRateDelta: after.passRate - before.passRate,
      judgeScoreMeanDelta:
        before.judgeScoreMean === null || after.judgeScoreMean === null
          ? null
          : after.judgeScoreMean - before.judgeScoreMean,
      averageDurationMsDelta: after.averageDurationMs - before.averageDurationMs
    },
    improvedCases: caseDeltas
      .filter((entry) => entry.deltaPassRate > 0)
      .sort((left, right) => right.deltaPassRate - left.deltaPassRate || left.caseId.localeCompare(right.caseId)),
    regressedCases: caseDeltas
      .filter((entry) => entry.deltaPassRate < 0)
      .sort((left, right) => left.deltaPassRate - right.deltaPassRate || left.caseId.localeCompare(right.caseId)),
    failureDeltas: [...failureCounts.entries()]
      .map(([name, counts]) => ({
        name,
        before: counts.before,
        after: counts.after,
        delta: counts.after - counts.before
      }))
      .filter((entry) => entry.delta !== 0)
      .sort((left, right) => Math.abs(right.delta) - Math.abs(left.delta) || left.name.localeCompare(right.name))
  };
}

export function buildOfficialRunFromResult(
  result: BenchmarkRunResult,
  labelOverride?: string
) {
  const flakeRate =
    result.caseResults.length === 0
      ? 0
      : result.caseResults.filter(
          (entry) => entry.passRate > 0 && entry.passRate < 1
        ).length / result.caseResults.length;
  const pathConsistency = average(
    result.caseResults.map((entry) => entry.pathConsistency)
  );
  const qualityScore = result.passRate * 100;
  const efficiencyScore = computeEfficiencyScore(result);

  return {
    timestamp: result.createdAt,
    git_sha: result.gitSha,
    suite_version: "cli-benchmark-v1",
    scoring_version: "cli-deterministic-v1",
    surface: result.canonicalSurface,
    label: labelOverride ?? result.label,
    provider: result.provider,
    model: result.model,
    judge_model: result.judgeModel,
    runs_per_case: result.runs,
    case_count: result.caseResults.length,
    metrics: {
      quality: {
        pass_rate: result.passRate,
        deterministic_pass_rate: result.passRate,
        judge_score_mean: result.judgeScoreMean ?? 0,
        judge_score_median: result.judgeScoreMedian ?? 0,
        judge_score_p10: result.judgeScoreP10 ?? 0,
        quality_score: qualityScore
      },
      reliability: {
        runs_per_case: result.runs,
        flake_rate: flakeRate,
        path_consistency: pathConsistency
      },
      efficiency: {
        latency_ms_mean: result.averageDurationMs,
        latency_ms_median: result.medianDurationMs,
        tokens_total_mean: 0,
        tool_calls_mean: result.averageToolCalls,
        iterations_mean: result.averageAssistantMessages,
        estimated_cost_mean: 0,
        cost_per_success: 0,
        latency_per_success: result.latencyPerSuccessMs,
        efficiency_score: efficiencyScore,
        value_score: (qualityScore * efficiencyScore) / 100
      }
    },
    cases: result.caseResults.map((entry) => ({
      id: entry.caseId,
      pass_rate: entry.passRate,
      average_duration_ms: entry.averageDurationMs,
      median_duration_ms: entry.medianDurationMs,
      latency_per_success_ms: entry.latencyPerSuccessMs,
      average_assistant_messages: entry.averageAssistantMessages,
      average_tool_calls: entry.averageToolCalls,
      average_skill_invocations: entry.averageSkillInvocations,
      path_consistency: entry.pathConsistency,
      distinct_skills_invoked: entry.distinctSkillsInvoked,
      distinct_tools_used: entry.distinctToolsUsed,
      required_failure_counts: entry.requiredFailureCounts
    }))
  };
}

export function formatSurfaceLabel(surface: SurfaceName | CanonicalSurfaceName): string {
  if (surface === "frontend-flow") {
    return "flow";
  }
  if (surface === "frontend-app") {
    return "app";
  }
  if (surface === "frontend-script") {
    return "script";
  }
  return surface;
}

function computeEfficiencyScore(result: BenchmarkRunResult): number {
  const latencyFactor = 1 / (1 + result.averageDurationMs / 20000);
  const toolFactor = 1 / (1 + result.averageToolCalls / 10);
  const iterationFactor = 1 / (1 + result.averageAssistantMessages / 10);
  return ((latencyFactor + toolFactor + iterationFactor) / 3) * 100;
}

function average(values: number[]): number {
  if (values.length === 0) {
    return 0;
  }
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}

function slugifyTimestamp(value: string): string {
  return value.replaceAll(":", "-").replaceAll(".", "-");
}

function slugify(value: string): string {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "") || "default";
}
