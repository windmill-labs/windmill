import { loadSelectedCases } from "../../core/cases";
import { buildRunResult } from "../../core/results";
import { runSuite } from "../../core/runSuite";
import type { BenchmarkRunResult, ModeRunner } from "../../core/types";
import { emitFrontendBenchmarkProgress } from "./progress";
import { createAppModeRunner } from "../../modes/app";
import { createFlowModeRunner } from "../../modes/flow";
import { createScriptModeRunner } from "../../modes/script";
import { DEFAULT_JUDGE_MODEL } from "../../core/judge";
import { getFrontendRunModelLabel } from "../../modes/frontendCommon";

export type FrontendBenchmarkMode = "flow" | "app" | "script";

export async function runFrontendBenchmarkFromEnv(): Promise<BenchmarkRunResult> {
  const mode = parseMode(process.env.WMILL_FRONTEND_AI_EVAL_MODE);
  const caseIds = parseOptionalJsonStringArray(process.env.WMILL_FRONTEND_AI_EVAL_CASE_IDS);
  const runs = parsePositiveInteger(process.env.WMILL_FRONTEND_AI_EVAL_RUNS, "WMILL_FRONTEND_AI_EVAL_RUNS");
  const emitProgress = process.env.WMILL_FRONTEND_AI_EVAL_PROGRESS === "1";

  const selectedCases = await loadSelectedCases(mode, caseIds);
  const modeRunner = getModeRunner(mode);
  const caseResults = await runSuite({
    modeRunner,
    cases: selectedCases,
    runs,
    runModel: getFrontendRunModelLabel(),
    judgeModel: DEFAULT_JUDGE_MODEL,
    onProgress: emitProgress ? (event) => emitFrontendBenchmarkProgress(event) : undefined,
  });

  return buildRunResult({
    mode,
    runs,
    runModel: getFrontendRunModelLabel(),
    judgeModel: DEFAULT_JUDGE_MODEL,
    caseResults,
  });
}

function getModeRunner(mode: FrontendBenchmarkMode): ModeRunner<any, any, any> {
  switch (mode) {
    case "flow":
      return createFlowModeRunner();
    case "app":
      return createAppModeRunner();
    case "script":
      return createScriptModeRunner();
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
