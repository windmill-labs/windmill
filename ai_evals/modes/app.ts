import { loadAppFixture } from "../adapters/frontend/core/app/appFixtureLoader";
import { buildAppArtifacts } from "../core/appArtifacts";
import type { AppValidationSpec } from "../core/types";
import type { FrontendEvalModelConfig } from "../core/models";
import { validateAppState, type AppFilesState } from "../core/validators";
import type { BenchmarkArtifactFile, ModeRunner } from "../core/types";
import { runAppEval } from "../adapters/frontend/core/app/appEvalRunner";
import { getFrontendApiKey } from "./frontendCommon";
import type { WindmillBackendSettings } from "../core/windmillBackendSettings";

export function createAppModeRunner(
  modelConfig: FrontendEvalModelConfig,
  backendSettings: WindmillBackendSettings,
): ModeRunner<AppFilesState, AppFilesState, AppFilesState> {
  return {
    mode: "app",
    concurrency: 5,
    judgeThreshold: 80,
    async loadInitial(path) {
      return path ? await loadAppFixture(path) : undefined;
    },
    async loadExpected(path) {
      return path ? await loadAppFixture(path) : undefined;
    },
    async run(prompt, initial, context) {
      const result = await runAppEval(
        prompt,
        getFrontendApiKey(modelConfig.provider),
        {
          initialFrontend: initial?.frontend,
          initialBackend: initial?.backend,
          initialDatatables: initial?.datatables,
          maxIterations: context.evalCase?.runtime?.maxTurns,
          appContext: context.evalCase?.runtime?.appContext,
          provider: modelConfig.provider,
          model: modelConfig.model,
          backend: backendSettings,
          runContext: context,
        },
      );

      return {
        success: result.success,
        actual: result.files as AppFilesState,
        error: result.error,
        assistantMessageCount: result.assistantMessageCount,
        toolCallCount: result.toolCallCount,
        toolsUsed: result.toolsUsed,
        skillsInvoked: [],
        tokenUsage: result.tokenUsage,
        finalContextTokens: result.finalContextTokens,
      };
    },
    validate({ evalCase, actual, initial, expected, run }) {
      return validateAppState({
        actual,
        initial,
        expected,
        validate: evalCase.validate as AppValidationSpec | undefined,
        toolsUsed: run.toolsUsed,
      });
    },
    buildArtifacts(actual): BenchmarkArtifactFile[] {
      return buildAppArtifacts(actual);
    },
  };
}
