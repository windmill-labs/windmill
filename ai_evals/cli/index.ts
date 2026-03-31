#!/usr/bin/env bun

import {
  cleanupWorkspace,
  loadCliArtifactEvalCases,
  runCliArtifactEvalCase
} from "../adapters/cli/artifact-eval";
import {
  loadCliVariantById,
  loadCliVariants,
  snapshotCliVariant,
  type CliVariant
} from "../adapters/cli/variants";

type CommandName =
  | "run"
  | "list-cases"
  | "list-variants"
  | "snapshot-variant"
  | "compare"
  | "history";
type SurfaceName = "cli";

interface ParsedArgs {
  command: CommandName;
  surface?: string;
  caseIds: string[];
  variantIds: string[];
  description?: string;
  runs: number;
  json: boolean;
  keepWorkspace: boolean;
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
}

interface AggregateMetrics {
  totalRuns: number;
  passedRuns: number;
  passRate: number;
  averageDurationMs: number;
  averageAssistantMessages: number;
  averageToolCalls: number;
  averageSkillInvocations: number;
  distinctSkillsInvoked: string[];
  distinctToolsUsed: string[];
  requiredFailureCounts: Array<{ name: string; count: number }>;
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
      throw new Error(
        `'${args.command}' is not implemented yet in the repo-level benchmark CLI`
      );
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
      const labeledVariants = labelVariantSelections(variants);
      const variantResults = [];

      for (const labeledVariant of labeledVariants) {
        const caseResults = [];
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
            averageAssistantMessages: execution.aggregate.averageAssistantMessages,
            averageToolCalls: execution.aggregate.averageToolCalls,
            averageSkillInvocations: execution.aggregate.averageSkillInvocations,
            distinctSkillsInvoked: execution.aggregate.distinctSkillsInvoked,
            distinctToolsUsed: execution.aggregate.distinctToolsUsed,
            requiredFailureCounts: execution.aggregate.requiredFailureCounts
          });
        }

        const aggregate = aggregateAttempts(allAttempts);

        variantResults.push({
          label: labeledVariant.label,
          variant: labeledVariant.variant.id,
          totalCases: caseResults.length,
          fullyPassedCases: caseResults.filter(
            (entry) => entry.passedRuns === entry.totalRuns
          ).length,
          totalRuns: aggregate.totalRuns,
          passedRuns: aggregate.passedRuns,
          passRate: aggregate.passRate,
          averageDurationMs: aggregate.averageDurationMs,
          averageAssistantMessages: aggregate.averageAssistantMessages,
          averageToolCalls: aggregate.averageToolCalls,
          averageSkillInvocations: aggregate.averageSkillInvocations,
          distinctSkillsInvoked: aggregate.distinctSkillsInvoked,
          distinctToolsUsed: aggregate.distinctToolsUsed,
          requiredFailureCounts: aggregate.requiredFailureCounts,
          caseResults
        });
      }

      const payload = {
        command: "compare",
        surface,
        caseIds: selectedCases.map((entry) => entry.id),
        runs: args.runs,
        variants: variantResults
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
  return {
    attempt,
    passed: result.passed,
    durationMs: result.run.durationMs,
    assistantMessageCount: result.run.assistantMessageCount,
    toolCallCount: result.run.toolsUsed.length,
    skillInvocationCount: result.run.skillsInvoked.length,
    skillsInvoked: uniqueStrings(result.run.skillsInvoked),
    toolsUsed: uniqueStrings(result.run.toolsUsed.map((tool) => tool.tool)),
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
    }))
  };
}

function aggregateAttempts(attempts: AttemptSummary[]): AggregateMetrics {
  const requiredFailureCounts = new Map<string, number>();

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
    averageAssistantMessages: average(attempts.map((attempt) => attempt.assistantMessageCount)),
    averageToolCalls: average(attempts.map((attempt) => attempt.toolCallCount)),
    averageSkillInvocations: average(attempts.map((attempt) => attempt.skillInvocationCount)),
    distinctSkillsInvoked: uniqueStrings(attempts.flatMap((attempt) => attempt.skillsInvoked)),
    distinctToolsUsed: uniqueStrings(attempts.flatMap((attempt) => attempt.toolsUsed)),
    requiredFailureCounts: [...requiredFailureCounts.entries()]
      .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
      .map(([name, count]) => ({ name, count }))
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
    keepWorkspace: false
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
  if (surface !== "cli") {
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
  variants: Array<{
    label: string;
    variant: string;
    totalCases: number;
    fullyPassedCases: number;
    totalRuns: number;
    passedRuns: number;
    passRate: number;
    averageDurationMs: number;
    averageAssistantMessages: number;
    averageToolCalls: number;
    averageSkillInvocations: number;
    caseResults: Array<{
      caseId: string;
      totalRuns: number;
      passedRuns: number;
      passRate: number;
      averageDurationMs: number;
      averageAssistantMessages: number;
      averageToolCalls: number;
      averageSkillInvocations: number;
      requiredFailureCounts: Array<{ name: string; count: number }>;
    }>;
  }>;
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

function printHelp() {
  process.stdout.write(
    [
      "Usage:",
      "  cd ai_evals && bun run cli -- list-cases --surface cli [--json]",
      "  cd ai_evals && bun run cli -- list-variants --surface cli [--json]",
      "  cd ai_evals && bun run cli -- snapshot-variant --surface cli --variant <id> [--description <text>] [--json]",
      "  cd ai_evals && bun run cli -- run --surface cli --case <id> [--variant <id>] [--runs <n>] [--json] [--keep-workspace]",
      "  cd ai_evals && bun run cli -- compare --surface cli [--case <id> ...] [--variant <id> ...] [--runs <n>] [--json]",
      "  cd ai_evals && bun run cli -- history",
      "",
      "Current support:",
      "  surfaces: cli"
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
