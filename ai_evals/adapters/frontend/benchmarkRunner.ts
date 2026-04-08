import { loadSelectedCases } from "../../core/cases";
import {
  formatRunModelLabel,
  getFrontendEvalModel,
  resolveEvalModel,
} from "../../core/models";
import { buildRunResult } from "../../core/results";
import { runSuite } from "../../core/runSuite";
import type { BenchmarkRunResult, ModeRunner } from "../../core/types";
import { emitFrontendBenchmarkProgress } from "./progress";
import { createAppModeRunner } from "../../modes/app";
import { createFlowModeRunner } from "../../modes/flow";
import { createScriptModeRunner } from "../../modes/script";
import { DEFAULT_JUDGE_MODEL } from "../../core/judge";

export type FrontendBenchmarkMode = "flow" | "app" | "script";

export async function runFrontendBenchmarkFromEnv(): Promise<BenchmarkRunResult> {
  const mode = parseMode(process.env.WMILL_FRONTEND_AI_EVAL_MODE);
  const caseIds = parseOptionalJsonStringArray(process.env.WMILL_FRONTEND_AI_EVAL_CASE_IDS);
  const runs = parsePositiveInteger(process.env.WMILL_FRONTEND_AI_EVAL_RUNS, "WMILL_FRONTEND_AI_EVAL_RUNS");
  const emitProgress = process.env.WMILL_FRONTEND_AI_EVAL_PROGRESS === "1";
  const verbose = process.env.WMILL_FRONTEND_AI_EVAL_VERBOSE === "1";
  const model = resolveEvalModel(mode, process.env.WMILL_FRONTEND_AI_EVAL_MODEL);

  const selectedCases = await loadSelectedCases(mode, caseIds);
  const modeRunner = getModeRunner(mode, getFrontendEvalModel(model));
  const runModel = formatRunModelLabel(mode, model);
  const caseResults = await runSuite({
    modeRunner,
    cases: selectedCases,
    runs,
    runModel,
    judgeModel: DEFAULT_JUDGE_MODEL,
    concurrency: verbose ? 1 : undefined,
    verbose,
    onProgress: emitProgress ? (event) => emitFrontendBenchmarkProgress(event) : undefined,
  });

  return buildRunResult({
    mode,
    runs,
    runModel,
    judgeModel: DEFAULT_JUDGE_MODEL,
    caseResults,
  });
}

function getModeRunner(
  mode: FrontendBenchmarkMode,
  model: ReturnType<typeof getFrontendEvalModel>
): ModeRunner<any, any, any> {
  switch (mode) {
    case "flow":
      return createFlowModeRunner(model);
    case "app":
      return createAppModeRunner(model);
    case "script":
      return createScriptModeRunner(model);
  }
}

function parseMode(value: string | undefined): FrontendBenchmarkMode {
  if (value === "flow" || value === "app" || value === "script") {
    return value;
  }
  throw new Error(`Unsupported frontend benchmark mode: ${String(value)}`);
}

function parseOptionalJsonStringArray(value: string | undefined): string[] {
  if (!value) {
    return [];
  }
  const parsed = JSON.parse(value) as unknown;
  if (!Array.isArray(parsed) || parsed.some((entry) => typeof entry !== "string")) {
    throw new Error("WMILL_FRONTEND_AI_EVAL_CASE_IDS must be a JSON string array");
  }
  return parsed;
}

function parsePositiveInteger(value: string | undefined, envName: string): number {
  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed <= 0) {
    throw new Error(`${envName} must be a positive integer`);
  }
  return parsed;
}
