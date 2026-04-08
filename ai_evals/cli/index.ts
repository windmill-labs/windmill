#!/usr/bin/env bun

import { Command, InvalidArgumentError } from "commander";
import { loadCases, loadSelectedCases } from "../core/cases";
import {
  EVAL_MODELS,
  formatRunModelLabel,
  getCliEvalModel,
  getEvalModelHelpText,
  resolveEvalModel,
} from "../core/models";
import {
  buildRunResult,
  formatRunSummary,
  resolveRunOutputPath,
  writeRunArtifacts,
  writeRunResult,
} from "../core/results";
import { runSuite } from "../core/runSuite";
import { EVAL_MODES, type EvalMode } from "../core/types";
import { DEFAULT_JUDGE_MODEL } from "../core/judge";
import { createCliModeRunner } from "../modes/cli";
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
        "  bun run cli -- models",
        "  bun run cli -- cases",
        "  bun run cli -- cases flow",
        "  bun run cli -- run flow",
        "  bun run cli -- run flow --model 4o",
        "  bun run cli -- run flow flow-test0-sum-two-numbers --verbose",
        "  bun run cli -- run flow flow-test5-simple-modification --runs 3",
        "  bun run cli -- run cli bun-hello-script",
        "",
        "Models:",
        getEvalModelHelpText(),
      ].join("\n")
    );

  program
    .command("models")
    .description("List available model aliases")
    .action(() => {
      handleModels();
    });

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
    .option("--model <name>", `model alias (${EVAL_MODELS.map((entry) => entry.id).join(", ")})`)
    .option("--verbose", "stream assistant output during frontend runs")
    .action(
      async (
        mode: EvalMode,
        caseIds: string[],
        options: {
          runs: number;
          output?: string;
          model?: string;
          verbose?: boolean;
        }
      ) => {
        await handleRun({
          mode,
          caseIds,
          runs: options.runs,
          outputPath: options.output,
          model: options.model,
          verbose: options.verbose ?? false,
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

function handleModels() {
  process.stdout.write("Available models\n");
  for (const model of EVAL_MODELS) {
    const supports = [
      ...(model.frontend ? ["flow", "script", "app"] : []),
      ...(model.cli ? ["cli"] : []),
    ];
    const aliases = [model.id, ...model.aliases.filter((alias) => alias !== model.id)];
    process.stdout.write(`- ${model.id}: ${model.label}\n`);
    process.stdout.write(`  aliases: ${aliases.join(", ")}\n`);
    process.stdout.write(`  modes: ${supports.join(", ")}\n`);
  }
  process.stdout.write(`\nJudge model: ${DEFAULT_JUDGE_MODEL}\n`);
}

async function handleRun(input: {
  mode: EvalMode;
  caseIds: string[];
  runs: number;
  outputPath?: string;
  model?: string;
  verbose: boolean;
}) {
  const selectedCases = await loadSelectedCases(input.mode, input.caseIds);
  const model = resolveEvalModel(input.mode, input.model);
  const runModel = formatRunModelLabel(input.mode, model);
  process.stderr.write(`Starting ${input.mode} benchmark...\n`);

  const result =
    input.mode === "cli"
      ? await runCliBenchmark(selectedCases, input.runs, getCliEvalModel(model), runModel)
      : await runFrontendBenchmarkAdapter({
          mode: input.mode,
          caseIds: input.caseIds,
          runs: input.runs,
          model: model.id,
          verbose: input.verbose,
        });

  const resolvedOutputPath = resolveRunOutputPath(input.mode, input.outputPath);
  const artifactsPath = await writeRunArtifacts(result, resolvedOutputPath);
  const resultPath = await writeRunResult(result, resolvedOutputPath);
  process.stdout.write(`${formatRunSummary(result)}\n`);
  process.stdout.write(`Saved: ${resultPath}\n`);
  if (artifactsPath) {
    process.stdout.write(`Artifacts: ${artifactsPath}\n`);
  }
}

async function runCliBenchmark(
  cases: Awaited<ReturnType<typeof loadSelectedCases>>,
  runs: number,
  model: ReturnType<typeof getCliEvalModel>,
  runModel: string
) {
  const caseResults = await runSuite({
    modeRunner: createCliModeRunner(model),
    cases,
    runs,
    runModel,
    judgeModel: DEFAULT_JUDGE_MODEL,
  });

  return buildRunResult({
    mode: "cli",
    runs,
    runModel,
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
