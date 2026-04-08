#!/usr/bin/env bun

import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { Command, InvalidArgumentError, Option } from "commander";
import {
  cleanupWorkspace,
  loadCliArtifactEvalCases,
  runCliArtifactEvalCase,
  type CliArtifactEvalCase,
  type CliGuidanceConfig,
} from "../adapters/cli/artifact-eval";
import {
  CLI_BENCHMARK_MODEL,
  CLI_BENCHMARK_PROVIDER,
} from "../adapters/cli/runtime";
import {
  runFrontendBenchmarkAdapter,
  type FrontendAdapterPayload,
  type FrontendBenchmarkConfig as FrontendAdapterConfig,
} from "../adapters/frontend/runtime";
import {
  loadEvalCaseSummaries,
  type EvalSurfaceName,
} from "../adapters/shared/evalCases";
import {
  appendOfficialRun,
  loadSummaryHistory,
} from "../history/writer.mjs";
import {
  buildBenchmarkRunDiff,
  buildOfficialRunFromResult,
  formatSurfaceLabel,
  readBenchmarkRunResult,
  writeBenchmarkRunResult,
  type AggregateMetrics,
  type AttemptSummary,
  type BenchmarkCaseResult,
  type BenchmarkRunDiff,
  type BenchmarkRunResult,
  type CanonicalSurfaceName,
  type SurfaceName,
} from "./results";

const USER_SURFACES = ["cli", "flow", "app", "script"] as const;
const PROVIDERS = ["anthropic", "openai"] as const;

async function main() {
  const program = new Command()
    .name("bun run cli --")
    .description("Run AI evals on the current checkout and diff saved results")
    .usage("<command> [options]")
    .showHelpAfterError()
    .showSuggestionAfterError()
    .addHelpText(
      "after",
      [
        "",
        "Examples:",
        "  bun run cli -- list-cases",
        "  bun run cli -- run --surface flow --runs 3",
        "  bun run cli -- run --surface cli --skills-source ./system_prompts/auto-generated/skills",
        "  bun run cli -- diff-results ai_evals/results/before.json ai_evals/results/after.json",
        "  bun run cli -- promote-result ai_evals/results/latest.json --label main",
      ].join("\n")
    );

  program
    .command("list-cases")
    .description("List benchmark cases")
    .addOption(createOptionalSurfaceOption())
    .option("--json", "print machine-readable output")
    .action(async (options: { surface?: SurfaceName; json?: boolean }) => {
      await handleListCases(options.surface, options.json ?? false);
    });

  program
    .command("run")
    .description("Run the current checkout on one surface and save a local result")
    .addOption(createRequiredSurfaceOption())
    .option("--case <id>", "case id (repeatable)", collectOptionValues)
    .option("--runs <n>", "number of runs per case", parsePositiveInteger, 1)
    .option("--output <path>", "write the local result to this path")
    .option("--label <label>", "label to store with the result")
    .option("--json", "print machine-readable output")
    .option("--keep-workspace", "keep the final CLI workspace for inspection")
    .option("--skills-source <path>", "override the CLI skills source")
    .option("--agents-source <path>", "override AGENTS.md for CLI evals")
    .option("--claude-source <path>", "override CLAUDE.md for CLI evals")
    .addOption(createProviderOption())
    .option("--model <model>", "override the frontend chat model")
    .option("--system-prompt-file <path>", "replace the frontend system prompt from a file")
    .option(
      "--append-system-prompt-file <path>",
      "append extra instructions to the default frontend system prompt from a file"
    )
    .action(
      async (options: {
        surface: SurfaceName;
        case: string[];
        runs: number;
        output?: string;
        label?: string;
        json?: boolean;
        keepWorkspace?: boolean;
        skillsSource?: string;
        agentsSource?: string;
        claudeSource?: string;
        provider?: (typeof PROVIDERS)[number];
        model?: string;
        systemPromptFile?: string;
        appendSystemPromptFile?: string;
      }) => {
        await handleRun({
          surface: options.surface,
          caseIds: options.case ?? [],
          runs: options.runs,
          outputPath: options.output,
          label: options.label,
          json: options.json ?? false,
          keepWorkspace: options.keepWorkspace ?? false,
          skillsSourcePath: options.skillsSource,
          agentsSourcePath: options.agentsSource,
          claudeSourcePath: options.claudeSource,
          provider: options.provider,
          model: options.model,
          systemPromptFile: options.systemPromptFile,
          appendSystemPromptFile: options.appendSystemPromptFile,
        });
      }
    );

  program
    .command("diff-results")
    .description("Diff two saved benchmark results")
    .argument("<before>", "path to the earlier result JSON")
    .argument("<after>", "path to the later result JSON")
    .option("--json", "print machine-readable output")
    .action(async (before: string, after: string, options: { json?: boolean }) => {
      await handleDiffResults(before, after, options.json ?? false);
    });

  program
    .command("promote-result")
    .description("Promote a saved local result into official history")
    .argument("<result>", "path to a saved result JSON")
    .option("--label <label>", "override the label written into official history")
    .option("--json", "print machine-readable output")
    .action(async (resultPath: string, options: { label?: string; json?: boolean }) => {
      await handlePromoteResult(resultPath, options.label, options.json ?? false);
    });

  program
    .command("history")
    .description("Show recent official benchmark runs")
    .option("--limit <n>", "number of entries to show", parsePositiveInteger, 10)
    .option("--json", "print machine-readable output")
    .action(async (options: { limit: number; json?: boolean }) => {
      await handleHistory(options.limit, options.json ?? false);
    });

  await program.parseAsync(process.argv);
}

async function handleListCases(surface: SurfaceName | undefined, json: boolean) {
  const selectedSurfaces = surface ? [surface] : [...USER_SURFACES];
  const payload = selectedSurfaces.map((entry) => ({
    surface: entry,
    cases: loadEvalCaseSummaries(toEvalSurface(entry)).map((caseEntry) => ({
      id: caseEntry.id,
      title: caseEntry.title,
      tags: caseEntry.tags,
    })),
  }));

  if (json) {
    process.stdout.write(JSON.stringify({ surfaces: payload }, null, 2) + "\n");
    return;
  }

  for (const entry of payload) {
    process.stdout.write(`${entry.surface} (${entry.cases.length})\n`);
    for (const caseEntry of entry.cases) {
      process.stdout.write(`- ${caseEntry.id}: ${caseEntry.title}\n`);
    }
    process.stdout.write("\n");
  }
}

async function handleRun(input: {
  surface: SurfaceName;
  caseIds: string[];
  runs: number;
  outputPath?: string;
  label?: string;
  json: boolean;
  keepWorkspace: boolean;
  skillsSourcePath?: string;
  agentsSourcePath?: string;
  claudeSourcePath?: string;
  provider?: (typeof PROVIDERS)[number];
  model?: string;
  systemPromptFile?: string;
  appendSystemPromptFile?: string;
}) {
  if (input.systemPromptFile && input.appendSystemPromptFile) {
    throw new Error("Use either --system-prompt-file or --append-system-prompt-file, not both");
  }

  if (
    input.surface === "cli" &&
    (input.provider ||
      input.model ||
      input.systemPromptFile ||
      input.appendSystemPromptFile)
  ) {
    throw new Error(
      "--provider, --model, --system-prompt-file, and --append-system-prompt-file are frontend-only options"
    );
  }

  if (
    input.surface !== "cli" &&
    (input.keepWorkspace ||
      input.skillsSourcePath ||
      input.agentsSourcePath ||
      input.claudeSourcePath)
  ) {
    throw new Error(
      "--keep-workspace, --skills-source, --agents-source, and --claude-source are CLI-only options"
    );
  }

  const result =
    input.surface === "cli"
      ? await runCliSurface({
          surface: "cli",
          caseIds: input.caseIds,
          runs: input.runs,
          label: input.label,
          keepWorkspace: input.keepWorkspace,
          skillsSourcePath: input.skillsSourcePath,
          agentsSourcePath: input.agentsSourcePath,
          claudeSourcePath: input.claudeSourcePath,
        })
      : await runFrontendSurface({
          surface: input.surface,
          caseIds: input.caseIds,
          runs: input.runs,
          label: input.label,
          provider: input.provider,
          model: input.model,
          systemPromptFile: input.systemPromptFile,
          appendSystemPromptFile: input.appendSystemPromptFile,
        });

  const resultPath = await writeBenchmarkRunResult(result, input.outputPath);

  if (input.json) {
    process.stdout.write(
      JSON.stringify(
        {
          resultPath,
          result,
        },
        null,
        2
      ) + "\n"
    );
  } else {
    printRunSummary(result, resultPath);
  }

  if (result.passedRuns !== result.totalRuns) {
    process.exitCode = 1;
  }
}

async function handleDiffResults(beforePath: string, afterPath: string, json: boolean) {
  const before = await readBenchmarkRunResult(beforePath);
  const after = await readBenchmarkRunResult(afterPath);
  const diff = buildBenchmarkRunDiff(before, after);

  if (json) {
    process.stdout.write(
      JSON.stringify(
        {
          beforePath: path.resolve(beforePath),
          afterPath: path.resolve(afterPath),
          diff,
        },
        null,
        2
      ) + "\n"
    );
    return;
  }

  printDiffSummary(path.resolve(beforePath), path.resolve(afterPath), diff);
}

async function handlePromoteResult(
  resultPath: string,
  label: string | undefined,
  json: boolean
) {
  const result = await readBenchmarkRunResult(resultPath);
  const write = await appendOfficialRun(buildOfficialRunFromResult(result, label));
  const payload = {
    resultPath: path.resolve(resultPath),
    promotedLabel: label ?? result.label,
    runId: write.runId,
    runPath: write.runPath,
  };

  if (json) {
    process.stdout.write(JSON.stringify(payload, null, 2) + "\n");
    return;
  }

  process.stdout.write(`Promoted: ${payload.resultPath}\n`);
  process.stdout.write(`Label: ${payload.promotedLabel}\n`);
  process.stdout.write(`History: ${payload.runPath} (${payload.runId})\n`);
}

async function handleHistory(limit: number, json: boolean) {
  const entries = (await loadSummaryHistory()).slice(-limit).reverse();
  if (json) {
    process.stdout.write(JSON.stringify({ entries }, null, 2) + "\n");
    return;
  }

  process.stdout.write(`Official runs: ${entries.length}\n`);
  for (const entry of entries) {
    process.stdout.write(
      `- ${entry.timestamp} | ${formatSurfaceLabel(entry.surface)} | ${entry.variant_name} | ${formatPercent(entry.metrics.quality.pass_rate)} | flake ${formatPercent(entry.metrics.reliability.flake_rate)} | avg ${formatNumber(entry.metrics.efficiency.latency_ms_mean)} ms | ${entry.case_count} cases x ${entry.runs_per_case} runs\n`
    );
  }
}

async function runCliSurface(input: {
  surface: "cli";
  caseIds: string[];
  runs: number;
  label?: string;
  keepWorkspace: boolean;
  skillsSourcePath?: string;
  agentsSourcePath?: string;
  claudeSourcePath?: string;
}): Promise<BenchmarkRunResult> {
  const allCases = await loadCliArtifactEvalCases();
  const selectedCases = selectCliCases(allCases, input.caseIds);

  if (input.keepWorkspace && (input.runs !== 1 || selectedCases.length !== 1)) {
    throw new Error("--keep-workspace requires exactly one case and --runs 1");
  }

  const guidance = {
    label: input.label ?? deriveCliLabel(input),
    skillsSourcePath: resolveOptionalPath(input.skillsSourcePath),
    agentsSourcePath: resolveOptionalPath(input.agentsSourcePath),
    claudeSourcePath: resolveOptionalPath(input.claudeSourcePath),
  } satisfies CliGuidanceConfig;

  const caseResults: BenchmarkCaseResult[] = [];
  let keptWorkspaceDir: string | null = null;

  for (const evalCase of selectedCases) {
    const execution = await runCliCaseAttempts(
      evalCase,
      guidance,
      input.runs,
      input.keepWorkspace
    );
    keptWorkspaceDir = execution.keptWorkspaceDir ?? keptWorkspaceDir;
    caseResults.push({
      caseId: evalCase.id,
      ...execution.aggregate,
      pathConsistency: computePathConsistency(execution.attempts),
      attempts: execution.attempts,
    });
  }

  const aggregate = aggregateAttempts(caseResults.flatMap((entry) => entry.attempts));
  const result = buildRunResult({
    label: guidance.label,
    surface: "cli",
    provider: CLI_BENCHMARK_PROVIDER,
    model: CLI_BENCHMARK_MODEL,
    judgeModel: null,
    runs: input.runs,
    caseResults,
    config: {
      kind: "cli",
      skillsSourcePath: guidance.skillsSourcePath ?? null,
      agentsSourcePath: guidance.agentsSourcePath ?? null,
      claudeSourcePath: guidance.claudeSourcePath ?? null,
      keptWorkspaceDir,
    },
  });

  return result;
}

async function runFrontendSurface(input: {
  surface: Exclude<SurfaceName, "cli">;
  caseIds: string[];
  runs: number;
  label?: string;
  provider?: (typeof PROVIDERS)[number];
  model?: string;
  systemPromptFile?: string;
  appendSystemPromptFile?: string;
}): Promise<BenchmarkRunResult> {
  const systemPromptOverride = await readSystemPromptOverride(
    input.systemPromptFile,
    input.appendSystemPromptFile
  );
  const adapterConfig = {
    provider: input.provider,
    model: input.model,
    ...(systemPromptOverride ? { systemPrompt: systemPromptOverride } : {}),
  } satisfies FrontendAdapterConfig;

  const adapterResult = await runFrontendBenchmarkAdapter({
    surface: toFrontendCanonicalSurface(input.surface),
    caseIds: input.caseIds,
    runs: input.runs,
    config:
      adapterConfig.provider || adapterConfig.model || adapterConfig.systemPrompt
        ? adapterConfig
        : undefined,
  });

  const caseResults = adapterResult.caseResults.map((caseResult) => {
    const attempts = caseResult.attempts.map(summarizeFrontendAttempt);
    return {
      caseId: caseResult.caseId,
      ...aggregateAttempts(attempts),
      pathConsistency: computePathConsistency(attempts),
      attempts,
    } satisfies BenchmarkCaseResult;
  });

  return buildRunResult({
    label: input.label ?? deriveFrontendLabel(input, adapterResult),
    surface: input.surface,
    provider: adapterResult.provider,
    model: adapterResult.model,
    judgeModel: adapterResult.judgeModel,
    runs: input.runs,
    caseResults,
    config: {
      kind: "frontend",
      provider: adapterResult.provider,
      model: adapterResult.model,
      systemPromptMode: systemPromptOverride?.mode ?? "default",
      systemPromptFile: input.systemPromptFile
        ? path.resolve(input.systemPromptFile)
        : input.appendSystemPromptFile
          ? path.resolve(input.appendSystemPromptFile)
          : null,
    },
  });
}

function buildRunResult(input: {
  label: string;
  surface: SurfaceName;
  provider: string;
  model: string;
  judgeModel: string | null;
  runs: number;
  caseResults: BenchmarkCaseResult[];
  config: Record<string, unknown>;
}): BenchmarkRunResult {
  const aggregate = aggregateAttempts(input.caseResults.flatMap((entry) => entry.attempts));
  return {
    version: "ai-evals-run-v1",
    createdAt: new Date().toISOString(),
    gitSha: getGitSha(),
    label: input.label,
    surface: input.surface,
    canonicalSurface: toCanonicalSurface(input.surface),
    provider: input.provider,
    model: input.model,
    judgeModel: input.judgeModel,
    runs: input.runs,
    totalCases: input.caseResults.length,
    fullyPassedCases: input.caseResults.filter(
      (entry) => entry.passedRuns === entry.totalRuns
    ).length,
    caseIds: input.caseResults.map((entry) => entry.caseId),
    config: input.config,
    caseResults: input.caseResults,
    ...aggregate,
  };
}

async function runCliCaseAttempts(
  evalCase: CliArtifactEvalCase,
  guidance: CliGuidanceConfig,
  runs: number,
  keepWorkspace: boolean
): Promise<{
  attempts: AttemptSummary[];
  aggregate: AggregateMetrics;
  keptWorkspaceDir: string | null;
}> {
  const attempts: AttemptSummary[] = [];
  let keptWorkspaceDir: string | null = null;

  for (let attemptIndex = 0; attemptIndex < runs; attemptIndex += 1) {
    const result = await runCliArtifactEvalCase(evalCase, { guidance });
    attempts.push(summarizeCliAttempt(result, attemptIndex + 1));

    if (keepWorkspace && attemptIndex === runs - 1) {
      keptWorkspaceDir = result.workspaceDir;
    } else {
      await cleanupWorkspace(result.workspaceDir);
    }
  }

  return {
    attempts,
    aggregate: aggregateAttempts(attempts),
    keptWorkspaceDir,
  };
}

function summarizeCliAttempt(
  result: Awaited<ReturnType<typeof runCliArtifactEvalCase>>,
  attempt: number
): AttemptSummary {
  const skillsInvoked = uniqueStrings(result.run.skillsInvoked);
  const toolsUsed = uniqueStrings(result.run.toolsUsed.map((tool) => tool.tool));

  return {
    attempt,
    passed: result.passed,
    durationMs: result.run.durationMs,
    assistantMessageCount: result.run.assistantMessageCount,
    toolCallCount: result.run.toolsUsed.length,
    skillInvocationCount: result.run.skillsInvoked.length,
    skillsInvoked,
    toolsUsed,
    checks: result.checks.map((check) => ({
      name: check.name,
      passed: check.passed,
      required: check.required,
    })),
    requiredFailedChecks: result.checks
      .filter((check) => check.required !== false && !check.passed)
      .map((check) => check.name),
    expectedFiles: result.expectedFiles.map((file) => ({
      path: file.path,
      exists: file.exists,
    })),
    pathSignature: buildPathSignature(skillsInvoked, toolsUsed),
    judgeScore: null,
    error: null,
  };
}

function summarizeFrontendAttempt(
  attempt: FrontendAdapterPayload["caseResults"][number]["attempts"][number]
): AttemptSummary {
  const toolsUsed = uniqueStrings(attempt.toolsUsed);
  return {
    attempt: attempt.attempt,
    passed: attempt.passed,
    durationMs: attempt.durationMs,
    assistantMessageCount: attempt.assistantMessageCount,
    toolCallCount: attempt.toolCallCount,
    skillInvocationCount: 0,
    skillsInvoked: [],
    toolsUsed,
    checks: attempt.checks,
    requiredFailedChecks: attempt.requiredFailedChecks,
    expectedFiles: [],
    pathSignature: buildPathSignature([], toolsUsed),
    judgeScore: attempt.judgeScore,
    error: attempt.error,
  };
}

function aggregateAttempts(attempts: AttemptSummary[]): AggregateMetrics {
  const requiredFailureCounts = new Map<string, number>();
  const successfulAttempts = attempts.filter((attempt) => attempt.passed);
  const judgeScores = attempts
    .map((attempt) => attempt.judgeScore)
    .filter((score): score is number => typeof score === "number" && Number.isFinite(score));

  for (const attempt of attempts) {
    for (const failure of attempt.requiredFailedChecks) {
      requiredFailureCounts.set(failure, (requiredFailureCounts.get(failure) ?? 0) + 1);
    }
  }

  return {
    totalRuns: attempts.length,
    passedRuns: attempts.filter((attempt) => attempt.passed).length,
    passRate:
      attempts.length === 0
        ? 0
        : attempts.filter((attempt) => attempt.passed).length / attempts.length,
    averageDurationMs: average(attempts.map((attempt) => attempt.durationMs)),
    medianDurationMs: median(attempts.map((attempt) => attempt.durationMs)),
    latencyPerSuccessMs: average(successfulAttempts.map((attempt) => attempt.durationMs)),
    averageAssistantMessages: average(
      attempts.map((attempt) => attempt.assistantMessageCount)
    ),
    averageToolCalls: average(attempts.map((attempt) => attempt.toolCallCount)),
    averageSkillInvocations: average(
      attempts.map((attempt) => attempt.skillInvocationCount)
    ),
    distinctSkillsInvoked: uniqueStrings(attempts.flatMap((attempt) => attempt.skillsInvoked)),
    distinctToolsUsed: uniqueStrings(attempts.flatMap((attempt) => attempt.toolsUsed)),
    requiredFailureCounts: [...requiredFailureCounts.entries()]
      .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
      .map(([name, count]) => ({ name, count })),
    judgeScoreMean: judgeScores.length > 0 ? average(judgeScores) : null,
    judgeScoreMedian: judgeScores.length > 0 ? median(judgeScores) : null,
    judgeScoreP10: judgeScores.length > 0 ? percentile(judgeScores, 0.1) : null,
  };
}

async function readSystemPromptOverride(
  replacePath: string | undefined,
  appendPath: string | undefined
): Promise<FrontendAdapterConfig["systemPrompt"] | undefined> {
  if (replacePath) {
    return {
      mode: "replace",
      content: await readFile(path.resolve(replacePath), "utf8"),
    };
  }

  if (appendPath) {
    return {
      mode: "append",
      content: await readFile(path.resolve(appendPath), "utf8"),
    };
  }

  return undefined;
}

function selectCliCases(
  allCases: CliArtifactEvalCase[],
  caseIds: string[]
): CliArtifactEvalCase[] {
  if (caseIds.length === 0) {
    return allCases;
  }

  return caseIds.map((caseId) => {
    const evalCase = allCases.find((entry) => entry.id === caseId);
    if (!evalCase) {
      throw new Error(`Unknown CLI case: ${caseId}`);
    }
    return evalCase;
  });
}

function deriveCliLabel(input: {
  skillsSourcePath?: string;
  agentsSourcePath?: string;
  claudeSourcePath?: string;
}): string {
  return input.skillsSourcePath || input.agentsSourcePath || input.claudeSourcePath
    ? "custom-guidance"
    : "default";
}

function deriveFrontendLabel(
  input: {
    provider?: (typeof PROVIDERS)[number];
    model?: string;
    systemPromptFile?: string;
    appendSystemPromptFile?: string;
  },
  result: FrontendAdapterPayload
): string {
  if (input.systemPromptFile) {
    return "custom-system-prompt";
  }
  if (input.appendSystemPromptFile) {
    return "appended-system-prompt";
  }
  if (input.provider || input.model) {
    return `${result.provider}-${result.model}`;
  }
  return "default";
}

function createRequiredSurfaceOption(): Option {
  return new Option("--surface <surface>", "surface: cli, flow, app, or script")
    .argParser(parseSurface)
    .makeOptionMandatory(true);
}

function createOptionalSurfaceOption(): Option {
  return new Option("--surface <surface>", "surface: cli, flow, app, or script")
    .argParser(parseSurface);
}

function createProviderOption(): Option {
  return new Option("--provider <provider>", "frontend provider")
    .choices([...PROVIDERS]);
}

function parseSurface(value: string): SurfaceName {
  if (value === "cli") {
    return "cli";
  }
  if (value === "flow" || value === "frontend-flow") {
    return "flow";
  }
  if (value === "app" || value === "frontend-app") {
    return "app";
  }
  if (value === "script" || value === "frontend-script") {
    return "script";
  }
  throw new InvalidArgumentError(
    `Unsupported surface '${value}'. Use one of: ${USER_SURFACES.join(", ")}`
  );
}

function toCanonicalSurface(surface: SurfaceName): CanonicalSurfaceName {
  if (surface === "cli") {
    return "cli";
  }
  if (surface === "flow") {
    return "frontend-flow";
  }
  if (surface === "app") {
    return "frontend-app";
  }
  return "frontend-script";
}

function toFrontendCanonicalSurface(
  surface: Exclude<SurfaceName, "cli">
): Exclude<CanonicalSurfaceName, "cli"> {
  return toCanonicalSurface(surface) as Exclude<CanonicalSurfaceName, "cli">;
}

function toEvalSurface(surface: SurfaceName): EvalSurfaceName {
  return toCanonicalSurface(surface);
}

function collectOptionValues(value: string, previous: string[] = []): string[] {
  return [...previous, value];
}

function parsePositiveInteger(value: string): number {
  const parsed = Number.parseInt(value, 10);
  if (!Number.isFinite(parsed) || parsed < 1) {
    throw new InvalidArgumentError("must be a positive integer");
  }
  return parsed;
}

function resolveOptionalPath(inputPath: string | undefined): string | undefined {
  return inputPath ? path.resolve(inputPath) : undefined;
}

function getGitSha(): string {
  try {
    return execFileSync("git", ["rev-parse", "HEAD"], {
      cwd: fileURLToPath(new URL("../..", import.meta.url)),
      encoding: "utf8",
    }).trim();
  } catch {
    return "0000000";
  }
}

function buildPathSignature(skillsInvoked: string[], toolsUsed: string[]): string {
  return `skills:${skillsInvoked.join(",") || "(none)"}|tools:${toolsUsed.join(",") || "(none)"}`;
}

function computePathConsistency(attempts: AttemptSummary[]): number {
  if (attempts.length === 0) {
    return 0;
  }

  const counts = new Map<string, number>();
  for (const attempt of attempts) {
    counts.set(attempt.pathSignature, (counts.get(attempt.pathSignature) ?? 0) + 1);
  }
  return Math.max(...counts.values()) / attempts.length;
}

function printRunSummary(result: BenchmarkRunResult, resultPath: string) {
  process.stdout.write(`Surface: ${result.surface}\n`);
  process.stdout.write(`Label: ${result.label}\n`);
  process.stdout.write(`Saved: ${resultPath}\n`);
  const keptWorkspaceDir =
    typeof result.config.keptWorkspaceDir === "string"
      ? result.config.keptWorkspaceDir
      : null;
  if (keptWorkspaceDir) {
    process.stdout.write(`Workspace: ${keptWorkspaceDir}\n`);
  }
  process.stdout.write(
    `Pass rate: ${result.passedRuns}/${result.totalRuns} (${formatPercent(result.passRate)})\n`
  );
  process.stdout.write(
    `Cases fully passed: ${result.fullyPassedCases}/${result.totalCases}\n`
  );
  process.stdout.write(`Avg duration: ${formatNumber(result.averageDurationMs)} ms\n`);
  if (result.judgeScoreMean !== null) {
    process.stdout.write(`Mean judge score: ${formatNumber(result.judgeScoreMean)}\n`);
  }
  if (result.requiredFailureCounts.length > 0) {
    process.stdout.write("Top failures:\n");
    for (const failure of result.requiredFailureCounts.slice(0, 5)) {
      process.stdout.write(`- ${failure.name}: ${failure.count}\n`);
    }
  }
  process.stdout.write("Cases:\n");
  for (const caseResult of result.caseResults) {
    process.stdout.write(
      `- ${caseResult.caseId}: ${caseResult.passedRuns}/${caseResult.totalRuns} (${formatPercent(caseResult.passRate)})\n`
    );
  }
}

function printDiffSummary(
  beforePath: string,
  afterPath: string,
  diff: BenchmarkRunDiff
) {
  process.stdout.write(`Surface: ${diff.surface}\n`);
  process.stdout.write(`Before: ${beforePath} (${diff.beforeLabel})\n`);
  process.stdout.write(`After: ${afterPath} (${diff.afterLabel})\n`);
  process.stdout.write(
    `Fully passed cases delta: ${formatSignedInteger(diff.summary.fullyPassedCasesDelta)}\n`
  );
  process.stdout.write(
    `Pass rate delta: ${formatSignedPercent(diff.summary.passRateDelta)}\n`
  );
  process.stdout.write(
    `Avg duration delta: ${formatSignedNumber(diff.summary.averageDurationMsDelta)} ms\n`
  );
  if (diff.summary.judgeScoreMeanDelta !== null) {
    process.stdout.write(
      `Mean judge score delta: ${formatSignedNumber(diff.summary.judgeScoreMeanDelta)}\n`
    );
  }

  if (diff.improvedCases.length > 0) {
    process.stdout.write("Improved cases:\n");
    for (const entry of diff.improvedCases.slice(0, 5)) {
      process.stdout.write(`- ${entry.caseId}: ${formatSignedPercent(entry.deltaPassRate)}\n`);
    }
  }

  if (diff.regressedCases.length > 0) {
    process.stdout.write("Regressed cases:\n");
    for (const entry of diff.regressedCases.slice(0, 5)) {
      process.stdout.write(`- ${entry.caseId}: ${formatSignedPercent(entry.deltaPassRate)}\n`);
    }
  }

  if (diff.failureDeltas.length > 0) {
    process.stdout.write("Failure deltas:\n");
    for (const entry of diff.failureDeltas.slice(0, 5)) {
      process.stdout.write(`- ${entry.name}: ${formatSignedInteger(entry.delta)}\n`);
    }
  }
}

function average(values: number[]): number {
  if (values.length === 0) {
    return 0;
  }
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}

function median(values: number[]): number {
  if (values.length === 0) {
    return 0;
  }

  const sorted = [...values].sort((left, right) => left - right);
  const middle = Math.floor(sorted.length / 2);
  if (sorted.length % 2 === 0) {
    return (sorted[middle - 1] + sorted[middle]) / 2;
  }
  return sorted[middle];
}

function percentile(values: number[], ratio: number): number {
  if (values.length === 0) {
    return 0;
  }

  const sorted = [...values].sort((left, right) => left - right);
  const index = Math.max(
    0,
    Math.min(sorted.length - 1, Math.floor((sorted.length - 1) * ratio))
  );
  return sorted[index];
}

function uniqueStrings(values: string[]): string[] {
  return [...new Set(values)].sort((left, right) => left.localeCompare(right));
}

function formatPercent(value: number): string {
  return `${(value * 100).toFixed(1)}%`;
}

function formatSignedPercent(value: number): string {
  return `${value >= 0 ? "+" : ""}${(value * 100).toFixed(1)}%`;
}

function formatNumber(value: number): string {
  return value.toFixed(1);
}

function formatSignedNumber(value: number): string {
  return `${value >= 0 ? "+" : ""}${value.toFixed(1)}`;
}

function formatSignedInteger(value: number): string {
  return `${value >= 0 ? "+" : ""}${value}`;
}

main().catch((error) => {
  process.stderr.write(`${error.message}\n`);
  process.exit(1);
});
