import { loadSelectedCases } from "../../core/cases";
import { resolveBackendValidationSettings } from "../../core/backendValidation";
import {
  formatRunModelLabel,
  getFrontendEvalModel,
  resolveEvalModel,
} from "../../core/models";
import { buildRunResult } from "../../core/results";
import { runSuite } from "../../core/runSuite";
import type { BenchmarkRunResult, ModeRunner } from "../../core/types";
import { resolveWindmillBackendSettings } from "../../core/windmillBackendSettings";
import { emitFrontendBenchmarkProgress } from "./progress";
import { DEFAULT_JUDGE_MODEL } from "../../core/judge";

export type FrontendBenchmarkMode = "flow" | "app" | "script" | "global";

export async function runFrontendBenchmarkFromEnv(): Promise<BenchmarkRunResult> {
  const mode = parseMode(process.env.WMILL_FRONTEND_AI_EVAL_MODE);
  const caseIds = parseOptionalJsonStringArray(
    process.env.WMILL_FRONTEND_AI_EVAL_CASE_IDS,
  );
  const runs = parsePositiveInteger(
    process.env.WMILL_FRONTEND_AI_EVAL_RUNS,
    "WMILL_FRONTEND_AI_EVAL_RUNS",
  );
  const emitProgress = process.env.WMILL_FRONTEND_AI_EVAL_PROGRESS === "1";
  const verbose = process.env.WMILL_FRONTEND_AI_EVAL_VERBOSE === "1";
  const model = resolveEvalModel(
    mode,
    process.env.WMILL_FRONTEND_AI_EVAL_MODEL,
  );
  const backendValidation = resolveBackendValidationSettings({
    evalMode: mode,
    requestedMode: process.env.WMILL_FRONTEND_AI_EVAL_BACKEND_VALIDATION,
  });
  const backendSettings = resolveWindmillBackendSettings();

  const selectedCases = await loadSelectedCases(mode, caseIds);
  const modeRunner = await getModeRunner(
    mode,
    getFrontendEvalModel(model),
    backendValidation,
    backendSettings,
  );
  const runModel = formatRunModelLabel(mode, model);
  const caseResults = await runSuite({
    modeRunner,
    cases: selectedCases,
    runs,
    runModel,
    judgeModel: DEFAULT_JUDGE_MODEL,
    concurrency: verbose ? 1 : undefined,
    verbose,
    onProgress: emitProgress
      ? (event) => emitFrontendBenchmarkProgress(event)
      : undefined,
  });

  return buildRunResult({
    mode,
    runs,
    runModel,
    judgeModel: DEFAULT_JUDGE_MODEL,
    caseResults,
  });
}

async function getModeRunner(
  mode: FrontendBenchmarkMode,
  model: ReturnType<typeof getFrontendEvalModel>,
  backendValidation: ReturnType<typeof resolveBackendValidationSettings>,
  backendSettings: ReturnType<typeof resolveWindmillBackendSettings>,
): Promise<ModeRunner<any, any, any>> {
  switch (mode) {
    case "flow": {
      const { createFlowModeRunner } = await import("../../modes/flow");
      return createFlowModeRunner(model, backendValidation, backendSettings);
    }
    case "app": {
      const { createAppModeRunner } = await import("../../modes/app");
      return createAppModeRunner(model, backendSettings);
    }
    case "script": {
      const { createScriptModeRunner } = await import("../../modes/script");
      return createScriptModeRunner(
        model,
        backendValidation,
        backendSettings,
      );
    }
    case "global": {
      const { createGlobalModeRunner } = await import("../../modes/global");
      return createGlobalModeRunner(model, backendSettings);
    }
  }
}

function parseMode(value: string | undefined): FrontendBenchmarkMode {
  if (value === "flow" || value === "app" || value === "script" || value === "global") {
    return value;
  }
  throw new Error(`Unsupported frontend benchmark mode: ${String(value)}`);
}

function parseOptionalJsonStringArray(value: string | undefined): string[] {
  if (!value) {
    return [];
  }
  const parsed = JSON.parse(value) as unknown;
  if (
    !Array.isArray(parsed) ||
    parsed.some((entry) => typeof entry !== "string")
  ) {
    throw new Error(
      "WMILL_FRONTEND_AI_EVAL_CASE_IDS must be a JSON string array",
    );
  }
  return parsed;
}

function parsePositiveInteger(
  value: string | undefined,
  envName: string,
): number {
  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed <= 0) {
    throw new Error(`${envName} must be a positive integer`);
  }
  return parsed;
}
