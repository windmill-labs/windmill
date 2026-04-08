#!/usr/bin/env bun

import { Command, InvalidArgumentError } from "commander";
import { loadCases, loadSelectedCases } from "../core/cases";
import { buildRunResult, formatRunSummary, writeRunResult } from "../core/results";
import { runSuite } from "../core/runSuite";
import { EVAL_MODES, type EvalMode } from "../core/types";
import { DEFAULT_JUDGE_MODEL } from "../core/judge";
import { createCliModeRunner, getCliRunModelLabel } from "../modes/cli";
import { runFrontendBenchmarkAdapter } from "../adapters/frontend/runtime";

async function main() {
  const program = new Command()
    .name("bun run cli --")
    .description("Run AI eval cases against the current production prompts and guidance")
    .showHelpAfterError()
    .showSuggestionAfterError()
    .addHelpText(
      "after",
      [
        "",
        "Examples:",
        "  bun run cli -- cases",
        "  bun run cli -- cases flow",
        "  bun run cli -- run flow",
        "  bun run cli -- run flow flow-test5-simple-modification --runs 3",
        "  bun run cli -- run cli bun-hello-script",
      ].join("\n")
    );

  program
    .command("cases")
    .description("List available cases")
    .argument("[mode]", "cli, flow, script, or app", parseOptionalMode)
    .action(async (mode?: EvalMode) => {
      await handleCases(mode);
    });

  program
    .command("run")
    .description("Run one benchmark mode")
    .argument("<mode>", "cli, flow, script, or app", parseMode)
    .argument("[caseIds...]", "specific case ids to run")
    .option("--runs <n>", "number of attempts per case", parsePositiveInteger, 1)
    .option("--output <path>", "write the result JSON to this path")
    .action(
      async (
        mode: EvalMode,
        caseIds: string[],
        options: {
          runs: number;
          output?: string;
        }
      ) => {
        await handleRun({
          mode,
          caseIds,
          runs: options.runs,
          outputPath: options.output,
        });
      }
    );

  await program.parseAsync(process.argv);
}

async function handleCases(mode?: EvalMode) {
  const modes = mode ? [mode] : [...EVAL_MODES];

  for (const entry of modes) {
    const cases = await loadCases(entry);
    process.stdout.write(`${entry} (${cases.length})\n`);
    for (const evalCase of cases) {
      process.stdout.write(`- ${evalCase.id}\n`);
    }
    process.stdout.write("\n");
  }
}

async function handleRun(input: {
  mode: EvalMode;
  caseIds: string[];
  runs: number;
  outputPath?: string;
}) {
  const selectedCases = await loadSelectedCases(input.mode, input.caseIds);
  process.stderr.write(`Starting ${input.mode} benchmark...\n`);

  const result =
    input.mode === "cli"
      ? await runCliBenchmark(selectedCases, input.runs)
      : await runFrontendBenchmarkAdapter({
          mode: input.mode,
          caseIds: input.caseIds,
          runs: input.runs,
        });

  const resultPath = await writeRunResult(result, input.outputPath);
  process.stdout.write(`${formatRunSummary(result)}\n`);
  process.stdout.write(`Saved: ${resultPath}\n`);
}

async function runCliBenchmark(
  cases: Awaited<ReturnType<typeof loadSelectedCases>>,
  runs: number
) {
  const caseResults = await runSuite({
    modeRunner: createCliModeRunner(),
    cases,
    runs,
    runModel: getCliRunModelLabel(),
    judgeModel: DEFAULT_JUDGE_MODEL,
  });

  return buildRunResult({
    mode: "cli",
    runs,
    runModel: getCliRunModelLabel(),
    judgeModel: DEFAULT_JUDGE_MODEL,
    caseResults,
  });
}

function parseMode(value: string): EvalMode {
  if (EVAL_MODES.includes(value as EvalMode)) {
    return value as EvalMode;
  }
  throw new InvalidArgumentError(`mode must be one of: ${EVAL_MODES.join(", ")}`);
}

function parseOptionalMode(value: string | undefined): EvalMode | undefined {
  return value ? parseMode(value) : undefined;
}

function parsePositiveInteger(value: string): number {
  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed <= 0) {
    throw new InvalidArgumentError("must be a positive integer");
  }
  return parsed;
}

void main().catch((error) => {
  const message = error instanceof Error ? error.message : String(error);
  process.stderr.write(`${message}\n`);
  process.exit(1);
});
