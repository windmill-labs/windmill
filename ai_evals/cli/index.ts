#!/usr/bin/env bun

import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  cleanupWorkspace,
  loadCliArtifactEvalCases,
  runCliArtifactEvalCase
} from "../adapters/cli/artifact-eval";
import {
  CLI_BENCHMARK_MODEL,
  CLI_BENCHMARK_PROVIDER,
} from "../adapters/cli/runtime";
import {
  loadCliVariantById,
  loadCliVariants,
  snapshotCliVariant,
  type CliVariant
} from "../adapters/cli/variants";
import {
  appendOfficialRun,
  DEFAULT_HISTORY_DIR,
  loadHistoryRollup,
  loadLatestHistoryRollup,
  loadSummaryHistory
} from "../history/writer.mjs";
import {
  loadFrontendCases,
  loadFrontendVariants,
  type FrontendSurfaceName,
} from "../adapters/frontend/manifests";
import {
  runFrontendBenchmarkAdapter,
  type FrontendAdapterPayload,
} from "../adapters/frontend/runtime";

type CommandName =
  | "run"
  | "list-cases"
  | "list-variants"
  | "snapshot-variant"
  | "compare"
  | "history";
type SurfaceName = "cli" | FrontendSurfaceName;

interface ParsedArgs {
  command: CommandName;
  surface?: string;
  caseIds: string[];
  variantIds: string[];
  description?: string;
  runs: number;
  json: boolean;
  keepWorkspace: boolean;
  writeHistory: boolean;
  historyDir?: string;
  historyView: "latest" | "summary" | "surface" | "variant" | "model";
  limit: number;
}

interface AttemptSummary {
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

interface AggregateMetrics {
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

interface CompareCaseResult {
  caseId: string;
  totalRuns: number;
  passedRuns: number;
  passRate: number;
  averageDurationMs: number;
  medianDurationMs: number;
  latencyPerSuccessMs: number;
  averageAssistantMessages: number;
  averageToolCalls: number;
  averageSkillInvocations: number;
  pathConsistency: number;
  distinctSkillsInvoked: string[];
  distinctToolsUsed: string[];
  requiredFailureCounts: Array<{ name: string; count: number }>;
}

interface CompareVariantResult {
  label: string;
  variant: string;
  provider: string;
  model: string;
  judgeModel: string | null;
  totalCases: number;
  fullyPassedCases: number;
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
  caseResults: CompareCaseResult[];
}

async function main() {
  const args = parseArgs(process.argv.slice(2));

  switch (args.command) {
    case "list-cases":
      await handleListCases(args);
      return;
    case "list-variants":
      await handleListVariants(args);
      return;
    case "snapshot-variant":
      await handleSnapshotVariant(args);
      return;
    case "run":
      await handleRun(args);
      return;
    case "compare":
      await handleCompare(args);
      return;
    case "history":
      await handleHistory(args);
      return;
    default:
      printHelp();
      throw new Error(`Unknown command: ${String(args.command)}`);
  }
}

async function handleListCases(args: ParsedArgs) {
  const surface = requireSurface(args.surface);
  switch (surface) {
    case "cli": {
      const cases = await loadCliArtifactEvalCases();
      const payload = {
        surface,
        cases: cases.map((entry) => ({
          id: entry.id,
          description: entry.description ?? null
        }))
      };

      if (args.json) {
        process.stdout.write(JSON.stringify(payload, null, 2) + "\n");
        return;
      }

      process.stdout.write(`Surface: ${surface}\n`);
      for (const entry of payload.cases) {
        process.stdout.write(
          `- ${entry.id}${entry.description ? `: ${entry.description}` : ""}\n`
        );
      }
      return;
    }
    case "frontend-flow":
    case "frontend-app":
    case "frontend-script": {
      const cases = await loadFrontendCases(surface);
      const payload = {
        surface,
        cases: cases.map((entry) => ({
          id: entry.id,
          description: entry.title ?? null
        }))
      };

      if (args.json) {
        process.stdout.write(JSON.stringify(payload, null, 2) + "\n");
        return;
      }

      process.stdout.write(`Surface: ${surface}\n`);
      for (const entry of payload.cases) {
        process.stdout.write(
          `- ${entry.id}${entry.description ? `: ${entry.description}` : ""}\n`
        );
      }
      return;
    }
    default:
      assertNever(surface);
  }
}

async function handleListVariants(args: ParsedArgs) {
  const surface = requireSurface(args.surface);

  switch (surface) {
    case "cli": {
      const variants = await loadCliVariants();
      const payload = {
        surface,
        variants: variants.map((entry) => ({
          id: entry.id,
          description: entry.description ?? null
        }))
      };

      if (args.json) {
        process.stdout.write(JSON.stringify(payload, null, 2) + "\n");
        return;
      }

      process.stdout.write(`Surface: ${surface}\n`);
      for (const entry of payload.variants) {
        process.stdout.write(
          `- ${entry.id}${entry.description ? `: ${entry.description}` : ""}\n`
        );
      }
      return;
    }
    case "frontend-flow":
    case "frontend-app":
    case "frontend-script": {
      const variants = await loadFrontendVariants(surface);
      const payload = {
        surface,
        variants: variants.map((entry) => ({
          id: entry.id,
          description: entry.description ?? null
        }))
      };

      if (args.json) {
        process.stdout.write(JSON.stringify(payload, null, 2) + "\n");
        return;
      }

      process.stdout.write(`Surface: ${surface}\n`);
      for (const entry of payload.variants) {
        process.stdout.write(
          `- ${entry.id}${entry.description ? `: ${entry.description}` : ""}\n`
        );
      }
      return;
    }
    default:
      assertNever(surface);
  }
}

async function handleRun(args: ParsedArgs) {
  const surface = requireSurface(args.surface);
  const caseId = requireSingleCaseId(args.caseIds);
  if (args.keepWorkspace && args.runs !== 1) {
    throw new Error("--keep-workspace is only supported when --runs is 1");
  }
  if (args.writeHistory) {
    throw new Error("--write-history is currently supported on compare only");
  }

  switch (surface) {
    case "cli": {
      const cases = await loadCliArtifactEvalCases();
      const evalCase = cases.find((entry) => entry.id === caseId);
      if (!evalCase) {
        throw new Error(`Unknown CLI case: ${caseId}`);
      }
      const variant = await loadCliVariantById(requireSingleVariantId(args.variantIds, "baseline"));
      const execution = await runCaseAttempts(evalCase, variant, args.runs, args.keepWorkspace);
      const payload = {
        command: "run",
        surface,
        variant: variant.id,
        caseId: evalCase.id,
        runs: args.runs,
        passed: execution.aggregate.passedRuns === execution.aggregate.totalRuns,
        passedRuns: execution.aggregate.passedRuns,
        totalRuns: execution.aggregate.totalRuns,
        passRate: execution.aggregate.passRate,
        workspaceKept: args.keepWorkspace,
        workspaceDir: execution.keptWorkspaceDir,
        metrics: {
          averageDurationMs: execution.aggregate.averageDurationMs,
          averageAssistantMessages: execution.aggregate.averageAssistantMessages,
          averageToolCalls: execution.aggregate.averageToolCalls,
          averageSkillInvocations: execution.aggregate.averageSkillInvocations
        },
        distinctSkillsInvoked: execution.aggregate.distinctSkillsInvoked,
        distinctToolsUsed: execution.aggregate.distinctToolsUsed,
        requiredFailureCounts: execution.aggregate.requiredFailureCounts,
        attempts: execution.attempts
      };

      if (args.json) {
        process.stdout.write(JSON.stringify(payload, null, 2) + "\n");
      } else {
        printRunSummary(payload);
      }

      if (execution.aggregate.passedRuns !== execution.aggregate.totalRuns) {
        process.exitCode = 1;
      }
      return;
    }
    case "frontend-flow":
    case "frontend-app":
    case "frontend-script": {
      if (args.keepWorkspace) {
        throw new Error("--keep-workspace is not supported for frontend benchmark surfaces");
      }
      const variantId = requireSingleVariantId(args.variantIds, "baseline");
      const payload = await buildFrontendRunPayload({
        surface,
        caseId,
        variantId,
        runs: args.runs
      });

      if (args.json) {
        process.stdout.write(JSON.stringify(payload, null, 2) + "\n");
      } else {
        printRunSummary(payload);
      }

      if (!payload.passed) {
        process.exitCode = 1;
      }
      return;
    }
    default:
      assertNever(surface);
  }
}

async function handleSnapshotVariant(args: ParsedArgs) {
  const surface = requireSurface(args.surface);
  const variantId = requireSingleVariantId(args.variantIds, "");

  switch (surface) {
    case "cli": {
      const result = await snapshotCliVariant({
        variantId,
        description: args.description
      });

      const payload = {
        command: "snapshot-variant",
        surface,
        variant: result.variantId,
        description: result.description,
        manifestPath: result.manifestPath,
        snapshotDir: result.snapshotDir,
        usedOverrides: result.usedOverrides
      };

      if (args.json) {
        process.stdout.write(JSON.stringify(payload, null, 2) + "\n");
      } else {
        printSnapshotSummary(payload);
      }

      return;
    }
    case "frontend-flow":
    case "frontend-app":
    case "frontend-script":
      throw new Error("snapshot-variant is currently supported for the cli surface only");
    default:
      assertNever(surface);
  }
}

async function handleCompare(args: ParsedArgs) {
  const surface = requireSurface(args.surface);

  switch (surface) {
    case "cli": {
      const allCases = await loadCliArtifactEvalCases();
      const selectedCases =
        args.caseIds.length === 0
          ? allCases
          : args.caseIds.map((caseId) => {
              const evalCase = allCases.find((entry) => entry.id === caseId);
              if (!evalCase) {
                throw new Error(`Unknown CLI case: ${caseId}`);
              }
              return evalCase;
            });

      if (args.variantIds.length < 2) {
        throw new Error("compare requires at least two --variant values");
      }

      const variants = await Promise.all(
        args.variantIds.map((variantId) => loadCliVariantById(variantId))
      );
      if (args.writeHistory && new Set(variants.map((variant) => variant.id)).size !== variants.length) {
        throw new Error(
          "--write-history requires distinct variant ids so official snapshots stay unambiguous"
        );
      }
      const labeledVariants = labelVariantSelections(variants);
      const internalVariantResults: Array<CompareVariantResult & { allAttempts: AttemptSummary[] }> = [];

      for (const labeledVariant of labeledVariants) {
        const caseResults: CompareCaseResult[] = [];
        const allAttempts: AttemptSummary[] = [];

        for (const evalCase of selectedCases) {
          const execution = await runCaseAttempts(
            evalCase,
            labeledVariant.variant,
            args.runs,
            false
          );
          allAttempts.push(...execution.attempts);
          caseResults.push({
            caseId: evalCase.id,
            totalRuns: execution.aggregate.totalRuns,
            passedRuns: execution.aggregate.passedRuns,
            passRate: execution.aggregate.passRate,
            averageDurationMs: execution.aggregate.averageDurationMs,
            medianDurationMs: execution.aggregate.medianDurationMs,
            latencyPerSuccessMs: execution.aggregate.latencyPerSuccessMs,
            averageAssistantMessages: execution.aggregate.averageAssistantMessages,
            averageToolCalls: execution.aggregate.averageToolCalls,
            averageSkillInvocations: execution.aggregate.averageSkillInvocations,
            pathConsistency: computePathConsistency(execution.attempts),
            distinctSkillsInvoked: execution.aggregate.distinctSkillsInvoked,
            distinctToolsUsed: execution.aggregate.distinctToolsUsed,
            requiredFailureCounts: execution.aggregate.requiredFailureCounts
          });
        }

        const aggregate = aggregateAttempts(allAttempts);

        internalVariantResults.push({
          label: labeledVariant.label,
          variant: labeledVariant.variant.id,
          provider: CLI_BENCHMARK_PROVIDER,
          model: CLI_BENCHMARK_MODEL,
          judgeModel: null,
          totalCases: caseResults.length,
          fullyPassedCases: caseResults.filter(
            (entry) => entry.passedRuns === entry.totalRuns
          ).length,
          totalRuns: aggregate.totalRuns,
          passedRuns: aggregate.passedRuns,
          passRate: aggregate.passRate,
          averageDurationMs: aggregate.averageDurationMs,
          medianDurationMs: aggregate.medianDurationMs,
          latencyPerSuccessMs: aggregate.latencyPerSuccessMs,
          averageAssistantMessages: aggregate.averageAssistantMessages,
          averageToolCalls: aggregate.averageToolCalls,
          averageSkillInvocations: aggregate.averageSkillInvocations,
          distinctSkillsInvoked: aggregate.distinctSkillsInvoked,
          distinctToolsUsed: aggregate.distinctToolsUsed,
          requiredFailureCounts: aggregate.requiredFailureCounts,
          judgeScoreMean: aggregate.judgeScoreMean,
          judgeScoreMedian: aggregate.judgeScoreMedian,
          judgeScoreP10: aggregate.judgeScoreP10,
          caseResults,
          allAttempts
        });
      }

      const historyWrites = args.writeHistory
        ? await writeCompareHistorySnapshots({
            surface,
            runs: args.runs,
            variants: internalVariantResults,
            historyDir: args.historyDir
          })
        : [];

      const payload = {
        command: "compare",
        surface,
        caseIds: selectedCases.map((entry) => entry.id),
        runs: args.runs,
        variants: internalVariantResults.map(({ allAttempts: _allAttempts, ...variant }) => variant),
        historyWrites
      };

      if (args.json) {
        process.stdout.write(JSON.stringify(payload, null, 2) + "\n");
      } else {
        printCompareSummary(payload);
      }

      return;
    }
    case "frontend-flow":
    case "frontend-app":
    case "frontend-script": {
      if (args.variantIds.length < 2) {
        throw new Error("compare requires at least two --variant values");
      }
      if (args.writeHistory && new Set(args.variantIds).size !== args.variantIds.length) {
        throw new Error(
          "--write-history requires distinct variant ids so official snapshots stay unambiguous"
        );
      }

      const internalVariantResults = await buildFrontendCompareResults({
        surface,
        caseIds: args.caseIds,
        variantIds: args.variantIds,
        runs: args.runs
      });

      const historyWrites = args.writeHistory
        ? await writeCompareHistorySnapshots({
            surface,
            runs: args.runs,
            variants: internalVariantResults,
            historyDir: args.historyDir
          })
        : [];

      const payload = {
        command: "compare",
        surface,
        caseIds:
          args.caseIds.length === 0
            ? internalVariantResults[0]?.caseResults.map((entry) => entry.caseId) ?? []
            : args.caseIds,
        runs: args.runs,
        variants: internalVariantResults.map(({ allAttempts: _allAttempts, ...variant }) => variant),
        historyWrites
      };

      if (args.json) {
        process.stdout.write(JSON.stringify(payload, null, 2) + "\n");
      } else {
        printCompareSummary(payload);
      }

      return;
    }
    default:
      assertNever(surface);
  }
}

async function handleHistory(args: ParsedArgs) {
  const historyDir = args.historyDir ?? DEFAULT_HISTORY_DIR;

  switch (args.historyView) {
    case "latest": {
      const payload = await loadLatestHistoryRollup(historyDir);
      if (args.json) {
        process.stdout.write(JSON.stringify(payload, null, 2) + "\n");
      } else {
        printHistoryLatestSummary(payload);
      }
      return;
    }
    case "summary": {
      const summaries = await loadSummaryHistory(historyDir);
      const payload = {
        view: "summary",
        historyDir,
        totalEntries: summaries.length,
        entries: summaries.slice(-args.limit).reverse()
      };
      if (args.json) {
        process.stdout.write(JSON.stringify(payload, null, 2) + "\n");
      } else {
        printHistorySummary(payload);
      }
      return;
    }
    case "surface":
    case "variant":
    case "model": {
      const filename =
        args.historyView === "surface"
          ? "by_surface.json"
          : args.historyView === "variant"
            ? "by_variant.json"
            : "by_model.json";
      const rollup = await loadHistoryRollup(filename, historyDir);
      const payload = {
        view: args.historyView,
        historyDir,
        generatedAt: rollup.generatedAt,
        groups: rollup.groups
      };
      if (args.json) {
        process.stdout.write(JSON.stringify(payload, null, 2) + "\n");
      } else {
        printHistoryGroupSummary(payload, args.limit);
      }
      return;
    }
    default:
      assertNever(args.historyView);
  }
}

async function buildFrontendRunPayload(input: {
  surface: FrontendSurfaceName;
  caseId: string;
  variantId: string;
  runs: number;
}) {
  const compareResults = await buildFrontendCompareResults({
    surface: input.surface,
    caseIds: [input.caseId],
    variantIds: [input.variantId],
    runs: input.runs
  });
  const variant = compareResults[0];

  return {
    command: "run",
    surface: input.surface,
    variant: variant.variant,
    caseId: input.caseId,
    runs: input.runs,
    passed: variant.passedRuns === variant.totalRuns,
    passedRuns: variant.passedRuns,
    totalRuns: variant.totalRuns,
    passRate: variant.passRate,
    workspaceKept: false,
    workspaceDir: null,
    metrics: {
      averageDurationMs: variant.averageDurationMs,
      averageAssistantMessages: variant.averageAssistantMessages,
      averageToolCalls: variant.averageToolCalls,
      averageSkillInvocations: variant.averageSkillInvocations
    },
    distinctSkillsInvoked: variant.distinctSkillsInvoked,
    distinctToolsUsed: variant.distinctToolsUsed,
    requiredFailureCounts: variant.requiredFailureCounts,
    attempts: variant.allAttempts
  };
}

async function buildFrontendCompareResults(input: {
  surface: FrontendSurfaceName;
  caseIds: string[];
  variantIds: string[];
  runs: number;
}): Promise<Array<CompareVariantResult & { allAttempts: AttemptSummary[] }>> {
  const adapterResult = await runFrontendBenchmarkAdapter({
    surface: input.surface,
    caseIds: input.caseIds,
    variantIds: input.variantIds,
    runs: input.runs
  });

  return adapterResult.variants.map((variant) => {
    const caseResults = variant.caseResults.map((caseResult) => {
      const attempts = caseResult.attempts.map(summarizeFrontendAttempt);
      const aggregate = aggregateAttempts(attempts);
      return {
        caseId: caseResult.caseId,
        totalRuns: aggregate.totalRuns,
        passedRuns: aggregate.passedRuns,
        passRate: aggregate.passRate,
        averageDurationMs: aggregate.averageDurationMs,
        medianDurationMs: aggregate.medianDurationMs,
        latencyPerSuccessMs: aggregate.latencyPerSuccessMs,
        averageAssistantMessages: aggregate.averageAssistantMessages,
        averageToolCalls: aggregate.averageToolCalls,
        averageSkillInvocations: aggregate.averageSkillInvocations,
        pathConsistency: computePathConsistency(attempts),
        distinctSkillsInvoked: aggregate.distinctSkillsInvoked,
        distinctToolsUsed: aggregate.distinctToolsUsed,
        requiredFailureCounts: aggregate.requiredFailureCounts
      };
    });

    const allAttempts = variant.caseResults.flatMap((caseResult) =>
      caseResult.attempts.map(summarizeFrontendAttempt)
    );
    const aggregate = aggregateAttempts(allAttempts);

    return {
      label: variant.variant,
      variant: variant.variant,
      provider: variant.provider,
      model: variant.model,
      judgeModel: variant.judgeModel,
      totalCases: caseResults.length,
      fullyPassedCases: caseResults.filter((entry) => entry.passedRuns === entry.totalRuns)
        .length,
      totalRuns: aggregate.totalRuns,
      passedRuns: aggregate.passedRuns,
      passRate: aggregate.passRate,
      averageDurationMs: aggregate.averageDurationMs,
      medianDurationMs: aggregate.medianDurationMs,
      latencyPerSuccessMs: aggregate.latencyPerSuccessMs,
      averageAssistantMessages: aggregate.averageAssistantMessages,
      averageToolCalls: aggregate.averageToolCalls,
      averageSkillInvocations: aggregate.averageSkillInvocations,
      distinctSkillsInvoked: aggregate.distinctSkillsInvoked,
      distinctToolsUsed: aggregate.distinctToolsUsed,
      requiredFailureCounts: aggregate.requiredFailureCounts,
      judgeScoreMean: aggregate.judgeScoreMean,
      judgeScoreMedian: aggregate.judgeScoreMedian,
      judgeScoreP10: aggregate.judgeScoreP10,
      caseResults,
      allAttempts
    };
  });
}

function summarizeFrontendAttempt(
  attempt: FrontendAdapterPayload["variants"][number]["caseResults"][number]["attempts"][number]
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
    error: attempt.error
  };
}

async function runCaseAttempts(
  evalCase: Awaited<ReturnType<typeof loadCliArtifactEvalCases>>[number],
  variant: CliVariant,
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
    const result = await runCliArtifactEvalCase(evalCase, { variant });
    attempts.push(summarizeAttempt(result, attemptIndex + 1));

    if (keepWorkspace && attemptIndex === runs - 1) {
      keptWorkspaceDir = result.workspaceDir;
    } else {
      await cleanupWorkspace(result.workspaceDir);
    }
  }

  return {
    attempts,
    aggregate: aggregateAttempts(attempts),
    keptWorkspaceDir
  };
}

function summarizeAttempt(
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
      required: check.required
    })),
    requiredFailedChecks: result.checks
      .filter((check) => check.required !== false && !check.passed)
      .map((check) => check.name),
    expectedFiles: result.expectedFiles.map((file) => ({
      path: file.path,
      exists: file.exists
    })),
    pathSignature: buildPathSignature(skillsInvoked, toolsUsed),
    judgeScore: null,
    error: result.error ?? null
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
    passRate: attempts.length === 0 ? 0 : attempts.filter((attempt) => attempt.passed).length / attempts.length,
    averageDurationMs: average(attempts.map((attempt) => attempt.durationMs)),
    medianDurationMs: median(attempts.map((attempt) => attempt.durationMs)),
    latencyPerSuccessMs: average(successfulAttempts.map((attempt) => attempt.durationMs)),
    averageAssistantMessages: average(attempts.map((attempt) => attempt.assistantMessageCount)),
    averageToolCalls: average(attempts.map((attempt) => attempt.toolCallCount)),
    averageSkillInvocations: average(attempts.map((attempt) => attempt.skillInvocationCount)),
    distinctSkillsInvoked: uniqueStrings(attempts.flatMap((attempt) => attempt.skillsInvoked)),
    distinctToolsUsed: uniqueStrings(attempts.flatMap((attempt) => attempt.toolsUsed)),
    requiredFailureCounts: [...requiredFailureCounts.entries()]
      .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
      .map(([name, count]) => ({ name, count })),
    judgeScoreMean: judgeScores.length > 0 ? average(judgeScores) : null,
    judgeScoreMedian: judgeScores.length > 0 ? median(judgeScores) : null,
    judgeScoreP10: judgeScores.length > 0 ? percentile(judgeScores, 0.1) : null
  };
}

function parseArgs(argv: string[]): ParsedArgs {
  const [commandArg, ...rest] = argv;

  if (!commandArg || commandArg === "--help" || commandArg === "-h") {
    printHelp();
    process.exit(0);
  }

  if (!isCommandName(commandArg)) {
    throw new Error(`Unknown command: ${commandArg}`);
  }

  const parsed: ParsedArgs = {
    command: commandArg,
    caseIds: [],
    variantIds: [],
    description: undefined,
    runs: 1,
    json: false,
    keepWorkspace: false,
    writeHistory: false,
    historyDir: undefined,
    historyView: "latest",
    limit: 10
  };

  for (let index = 0; index < rest.length; index += 1) {
    const arg = rest[index];

    if (arg === "--surface") {
      parsed.surface = rest[index + 1];
      index += 1;
      continue;
    }

    if (arg === "--case") {
      parsed.caseIds.push(rest[index + 1]);
      index += 1;
      continue;
    }

    if (arg === "--variant") {
      parsed.variantIds.push(rest[index + 1]);
      index += 1;
      continue;
    }

    if (arg === "--description") {
      parsed.description = rest[index + 1];
      index += 1;
      continue;
    }

    if (arg === "--runs") {
      parsed.runs = parsePositiveInteger(rest[index + 1], "--runs");
      index += 1;
      continue;
    }

    if (arg === "--write-history") {
      parsed.writeHistory = true;
      continue;
    }

    if (arg === "--history-dir") {
      parsed.historyDir = rest[index + 1];
      index += 1;
      continue;
    }

    if (arg === "--view") {
      parsed.historyView = parseHistoryView(rest[index + 1]);
      index += 1;
      continue;
    }

    if (arg === "--limit") {
      parsed.limit = parsePositiveInteger(rest[index + 1], "--limit");
      index += 1;
      continue;
    }

    if (arg === "--json") {
      parsed.json = true;
      continue;
    }

    if (arg === "--keep-workspace") {
      parsed.keepWorkspace = true;
      continue;
    }

    if (arg === "--help" || arg === "-h") {
      printHelp();
      process.exit(0);
    }

    throw new Error(`Unknown argument: ${arg}`);
  }

  return parsed;
}

function isCommandName(value: string): value is CommandName {
  return (
    value === "run" ||
    value === "list-cases" ||
    value === "list-variants" ||
    value === "snapshot-variant" ||
    value === "compare" ||
    value === "history"
  );
}

function requireSurface(surface: string | undefined): SurfaceName {
  if (!surface) {
    throw new Error("Missing required --surface argument");
  }
  if (
    surface !== "cli" &&
    surface !== "frontend-flow" &&
    surface !== "frontend-app" &&
    surface !== "frontend-script"
  ) {
    throw new Error(`Unsupported surface for now: ${surface}`);
  }
  return surface;
}

function requireSingleCaseId(caseIds: string[]): string {
  if (caseIds.length === 0) {
    throw new Error("Missing required --case argument");
  }
  if (caseIds.length > 1) {
    throw new Error("run accepts only one --case value");
  }
  return caseIds[0];
}

function requireSingleVariantId(variantIds: string[], fallback: string): string {
  if (variantIds.length === 0) {
    if (!fallback) {
      throw new Error("Missing required --variant argument");
    }
    return fallback;
  }
  if (variantIds.length > 1) {
    throw new Error("this command accepts only one --variant value");
  }
  return variantIds[0];
}

function parsePositiveInteger(value: string | undefined, flagName: string): number {
  if (!value) {
    throw new Error(`Missing value for ${flagName}`);
  }

  const parsed = Number.parseInt(value, 10);
  if (!Number.isFinite(parsed) || parsed < 1) {
    throw new Error(`${flagName} must be a positive integer`);
  }

  return parsed;
}

function parseHistoryView(
  value: string | undefined
): ParsedArgs["historyView"] {
  if (
    value === "latest" ||
    value === "summary" ||
    value === "surface" ||
    value === "variant" ||
    value === "model"
  ) {
    return value;
  }

  throw new Error("--view must be one of: latest, summary, surface, variant, model");
}

function printRunSummary(payload: {
  surface: SurfaceName;
  variant: string;
  caseId: string;
  runs: number;
  passed: boolean;
  passedRuns: number;
  totalRuns: number;
  passRate: number;
  workspaceKept: boolean;
  workspaceDir: string | null;
  metrics: {
    averageDurationMs: number;
    averageAssistantMessages: number;
    averageToolCalls: number;
    averageSkillInvocations: number;
  };
  distinctSkillsInvoked: string[];
  distinctToolsUsed: string[];
  requiredFailureCounts: Array<{ name: string; count: number }>;
  attempts: AttemptSummary[];
}) {
  process.stdout.write(`Surface: ${payload.surface}\n`);
  process.stdout.write(`Variant: ${payload.variant}\n`);
  process.stdout.write(`Case: ${payload.caseId}\n`);
  process.stdout.write(`Runs: ${payload.runs}\n`);
  process.stdout.write(`Passed: ${payload.passed ? "yes" : "no"}\n`);
  process.stdout.write(
    `Pass rate: ${payload.passedRuns}/${payload.totalRuns} (${formatPercent(payload.passRate)})\n`
  );
  process.stdout.write(
    `Avg duration: ${formatNumber(payload.metrics.averageDurationMs)} ms\n`
  );
  process.stdout.write(
    `Avg assistant messages: ${formatNumber(payload.metrics.averageAssistantMessages)}\n`
  );
  process.stdout.write(
    `Avg tool calls: ${formatNumber(payload.metrics.averageToolCalls)}\n`
  );
  process.stdout.write(
    `Avg skill invocations: ${formatNumber(payload.metrics.averageSkillInvocations)}\n`
  );
  process.stdout.write(
    `Skills: ${payload.distinctSkillsInvoked.join(", ") || "(none)"}\n`
  );
  process.stdout.write(`Tools: ${payload.distinctToolsUsed.join(", ") || "(none)"}\n`);

  if (payload.workspaceKept && payload.workspaceDir) {
    process.stdout.write(`Workspace: ${payload.workspaceDir}\n`);
  }

  process.stdout.write("Attempts:\n");
  for (const attempt of payload.attempts) {
    process.stdout.write(
      `- run ${attempt.attempt}: ${attempt.passed ? "pass" : "fail"} | ${attempt.durationMs} ms | tools=${attempt.toolCallCount} | skills=${attempt.skillInvocationCount}\n`
    );
  }

  if (payload.requiredFailureCounts.length > 0) {
    process.stdout.write("Required failures:\n");
    for (const failure of payload.requiredFailureCounts) {
      process.stdout.write(`- ${failure.name}: ${failure.count}\n`);
    }
  }
}

function printCompareSummary(payload: {
  surface: SurfaceName;
  caseIds: string[];
  runs: number;
  variants: CompareVariantResult[];
  historyWrites: Array<{ variant: string; runId: string; runPath: string }>;
}) {
  process.stdout.write(`Surface: ${payload.surface}\n`);
  process.stdout.write(`Cases: ${payload.caseIds.join(", ")}\n`);
  process.stdout.write(`Runs per case: ${payload.runs}\n`);
  process.stdout.write("Variants:\n");

  for (const variant of payload.variants) {
    process.stdout.write(
      `- ${variant.label}: ${variant.fullyPassedCases}/${variant.totalCases} fully passed | ${variant.passedRuns}/${variant.totalRuns} runs passed (${formatPercent(variant.passRate)}) | avg ${formatNumber(variant.averageDurationMs)} ms | avg tools=${formatNumber(variant.averageToolCalls)}\n`
    );
    for (const caseResult of variant.caseResults) {
      process.stdout.write(
        `  ${caseResult.caseId}: ${caseResult.passedRuns}/${caseResult.totalRuns} (${formatPercent(caseResult.passRate)}) | avg ${formatNumber(caseResult.averageDurationMs)} ms | avg tools=${formatNumber(caseResult.averageToolCalls)}\n`
      );
      for (const failure of caseResult.requiredFailureCounts.slice(0, 3)) {
        process.stdout.write(`    fail ${failure.name}: ${failure.count}\n`);
      }
    }
  }

  if (payload.historyWrites.length > 0) {
    process.stdout.write("History writes:\n");
    for (const write of payload.historyWrites) {
      process.stdout.write(`- ${write.variant}: ${write.runPath} (${write.runId})\n`);
    }
  }
}

function printSnapshotSummary(payload: {
  surface: SurfaceName;
  variant: string;
  description: string;
  manifestPath: string;
  snapshotDir: string;
  usedOverrides: {
    skillsSourcePath?: string;
    agentsSourcePath?: string;
    claudeSourcePath?: string;
  };
}) {
  process.stdout.write(`Surface: ${payload.surface}\n`);
  process.stdout.write(`Variant: ${payload.variant}\n`);
  process.stdout.write(`Description: ${payload.description}\n`);
  process.stdout.write(`Manifest: ${payload.manifestPath}\n`);
  process.stdout.write(`Snapshot: ${payload.snapshotDir}\n`);
  process.stdout.write("Overrides:\n");
  process.stdout.write(
    `- skills: ${payload.usedOverrides.skillsSourcePath ?? "(generated default)"}\n`
  );
  process.stdout.write(
    `- agents: ${payload.usedOverrides.agentsSourcePath ?? "(generated default)"}\n`
  );
  process.stdout.write(
    `- claude: ${payload.usedOverrides.claudeSourcePath ?? "(generated default)"}\n`
  );
}

function labelVariantSelections(
  variants: CliVariant[]
): Array<{ label: string; variant: CliVariant }> {
  const seenCounts = new Map<string, number>();

  return variants.map((variant) => {
    const count = (seenCounts.get(variant.id) ?? 0) + 1;
    seenCounts.set(variant.id, count);

    return {
      label: count === 1 ? variant.id : `${variant.id}#${count}`,
      variant
    };
  });
}

async function writeCompareHistorySnapshots(input: {
  surface: SurfaceName;
  runs: number;
  variants: Array<CompareVariantResult & { allAttempts: AttemptSummary[] }>;
  historyDir?: string;
}) {
  const timestamp = new Date().toISOString();
  const gitSha = getGitSha();
  const writes = [];

  for (const variant of input.variants) {
    const result = await appendOfficialRun(
      buildOfficialRun({
        timestamp,
        gitSha,
        surface: input.surface,
        runs: input.runs,
        variant
      }),
      {
        historyDir: input.historyDir
      }
    );

    writes.push({
      variant: variant.variant,
      runId: result.runId,
      runPath: result.runPath
    });
  }

  return writes;
}

function buildOfficialRun(input: {
  timestamp: string;
  gitSha: string;
  surface: SurfaceName;
  runs: number;
  variant: CompareVariantResult & { allAttempts: AttemptSummary[] };
}) {
  const flakeRate =
    input.variant.caseResults.length === 0
      ? 0
      : input.variant.caseResults.filter(
          (entry) => entry.passRate > 0 && entry.passRate < 1
        ).length / input.variant.caseResults.length;
  const pathConsistency = average(
    input.variant.caseResults.map((entry) => entry.pathConsistency)
  );
  const qualityScore = input.variant.passRate * 100;
  const efficiencyScore = computeEfficiencyScore(input.variant);

  return {
    timestamp: input.timestamp,
    git_sha: input.gitSha,
    suite_version: "cli-benchmark-v1",
    scoring_version: "cli-deterministic-v1",
    surface: input.surface,
    variant_name: input.variant.variant,
    provider: input.variant.provider,
    model: input.variant.model,
    judge_model: input.variant.judgeModel,
    runs_per_case: input.runs,
    case_count: input.variant.caseResults.length,
    metrics: {
      quality: {
        pass_rate: input.variant.passRate,
        deterministic_pass_rate: input.variant.passRate,
        judge_score_mean: input.variant.judgeScoreMean ?? 0,
        judge_score_median: input.variant.judgeScoreMedian ?? 0,
        judge_score_p10: input.variant.judgeScoreP10 ?? 0,
        quality_score: qualityScore
      },
      reliability: {
        runs_per_case: input.runs,
        flake_rate: flakeRate,
        path_consistency: pathConsistency
      },
      efficiency: {
        latency_ms_mean: input.variant.averageDurationMs,
        latency_ms_median: input.variant.medianDurationMs,
        tokens_total_mean: 0,
        tool_calls_mean: input.variant.averageToolCalls,
        iterations_mean: input.variant.averageAssistantMessages,
        estimated_cost_mean: 0,
        cost_per_success: 0,
        latency_per_success: input.variant.latencyPerSuccessMs,
        efficiency_score: efficiencyScore,
        value_score: (qualityScore * efficiencyScore) / 100
      }
    },
    cases: input.variant.caseResults.map((entry) => ({
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

function computeEfficiencyScore(variant: CompareVariantResult): number {
  const latencyFactor = 1 / (1 + variant.averageDurationMs / 20000);
  const toolFactor = 1 / (1 + variant.averageToolCalls / 10);
  const iterationFactor = 1 / (1 + variant.averageAssistantMessages / 10);

  return ((latencyFactor + toolFactor + iterationFactor) / 3) * 100;
}

function getGitSha(): string {
  try {
    return execFileSync("git", ["rev-parse", "HEAD"], {
      cwd: fileURLToPath(new URL("../..", import.meta.url)),
      encoding: "utf8"
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

function printHistoryLatestSummary(payload: {
  generatedAt: string;
  latestRun: null | {
    timestamp: string;
    surface: string;
    variant_name: string;
    provider: string;
    model: string;
    metrics: {
      quality: { pass_rate: number };
      efficiency: { latency_ms_mean: number };
    };
  };
  latestBySurface: Record<string, { variant_name: string; metrics: { quality: { pass_rate: number } } }>;
}) {
  process.stdout.write(`Generated: ${payload.generatedAt}\n`);
  if (!payload.latestRun) {
    process.stdout.write("Latest run: none\n");
    return;
  }

  process.stdout.write(
    `Latest run: ${payload.latestRun.timestamp} | ${payload.latestRun.surface} | ${payload.latestRun.variant_name} | ${formatPercent(payload.latestRun.metrics.quality.pass_rate)} | avg ${formatNumber(payload.latestRun.metrics.efficiency.latency_ms_mean)} ms\n`
  );
  process.stdout.write("Latest by surface:\n");
  for (const [surface, summary] of Object.entries(payload.latestBySurface)) {
    process.stdout.write(
      `- ${surface}: ${summary.variant_name} (${formatPercent(summary.metrics.quality.pass_rate)})\n`
    );
  }
}

function printHistorySummary(payload: {
  view: "summary";
  historyDir: string;
  totalEntries: number;
  entries: Array<{
    timestamp: string;
    surface: string;
    variant_name: string;
    provider: string;
    model: string;
    runs_per_case: number;
    case_count: number;
    metrics: {
      quality: { pass_rate: number };
      reliability: { flake_rate: number };
      efficiency: { latency_ms_mean: number };
    };
  }>;
}) {
  process.stdout.write(`History: ${payload.historyDir}\n`);
  process.stdout.write(`Entries: ${payload.totalEntries}\n`);
  for (const entry of payload.entries) {
    process.stdout.write(
      `- ${entry.timestamp} | ${entry.surface} | ${entry.variant_name} | ${formatPercent(entry.metrics.quality.pass_rate)} | flake ${formatPercent(entry.metrics.reliability.flake_rate)} | avg ${formatNumber(entry.metrics.efficiency.latency_ms_mean)} ms | ${entry.case_count} cases x ${entry.runs_per_case} runs\n`
    );
  }
}

function printHistoryGroupSummary(
  payload: {
    view: "surface" | "variant" | "model";
    historyDir: string;
    generatedAt: string;
    groups: Record<string, Array<{
      timestamp: string;
      variant_name: string;
      surface: string;
      metrics: {
        quality: { pass_rate: number };
        efficiency: { latency_ms_mean: number };
      };
    }>>;
  },
  limit: number
) {
  process.stdout.write(`History: ${payload.historyDir}\n`);
  process.stdout.write(`View: ${payload.view}\n`);
  process.stdout.write(`Generated: ${payload.generatedAt}\n`);
  for (const [group, entries] of Object.entries(payload.groups)
    .sort((left, right) => left[0].localeCompare(right[0]))
    .slice(0, limit)) {
    const latest = entries[entries.length - 1];
    process.stdout.write(
      `- ${group}: ${entries.length} runs | latest ${latest.timestamp} | ${latest.variant_name} | ${formatPercent(latest.metrics.quality.pass_rate)} | avg ${formatNumber(latest.metrics.efficiency.latency_ms_mean)} ms\n`
    );
  }
}

function printHelp() {
  process.stdout.write(
    [
      "Usage:",
      "  cd ai_evals && bun run cli -- list-cases --surface cli [--json]",
      "  cd ai_evals && bun run cli -- list-cases --surface frontend-flow [--json]",
      "  cd ai_evals && bun run cli -- list-cases --surface frontend-app [--json]",
      "  cd ai_evals && bun run cli -- list-cases --surface frontend-script [--json]",
      "  cd ai_evals && bun run cli -- list-variants --surface cli [--json]",
      "  cd ai_evals && bun run cli -- list-variants --surface frontend-flow [--json]",
      "  cd ai_evals && bun run cli -- list-variants --surface frontend-app [--json]",
      "  cd ai_evals && bun run cli -- list-variants --surface frontend-script [--json]",
      "  cd ai_evals && bun run cli -- snapshot-variant --surface cli --variant <id> [--description <text>] [--json]",
      "  cd ai_evals && bun run cli -- run --surface <cli|frontend-flow|frontend-app|frontend-script> --case <id> [--variant <id>] [--runs <n>] [--json] [--keep-workspace]",
      "  cd ai_evals && bun run cli -- compare --surface <cli|frontend-flow|frontend-app|frontend-script> [--case <id> ...] [--variant <id> ...] [--runs <n>] [--write-history] [--history-dir <path>] [--json]",
      "  cd ai_evals && bun run cli -- history [--view latest|summary|surface|variant|model] [--limit <n>] [--history-dir <path>] [--json]",
      "",
      "Current support:",
      "  surfaces: cli, frontend-flow, frontend-app, frontend-script"
    ].join("\n") + "\n"
  );
}

function assertNever(value: never): never {
  throw new Error(`Unexpected value: ${String(value)}`);
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
  const index = Math.max(0, Math.min(sorted.length - 1, Math.floor((sorted.length - 1) * ratio)));
  return sorted[index];
}

function uniqueStrings(values: string[]): string[] {
  return [...new Set(values)].sort((left, right) => left.localeCompare(right));
}

function formatPercent(value: number): string {
  return `${(value * 100).toFixed(1)}%`;
}

function formatNumber(value: number): string {
  return value.toFixed(1);
}

main().catch((error) => {
  process.stderr.write(`${error.message}\n`);
  process.exit(1);
});
