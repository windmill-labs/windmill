import { loadAppFixture } from "../adapters/frontend/core/app/appFixtureLoader";
import { buildAppArtifacts } from "../core/appArtifacts";
import type { AppValidationSpec } from "../core/types";
import type { FrontendEvalModelConfig } from "../core/models";
import { validateAppState, type AppFilesState } from "../core/validators";
import type { BenchmarkArtifactFile, ModeRunner } from "../core/types";
import { runAppEval } from "../adapters/frontend/core/app/appEvalRunner";
import {
  DEFAULT_FRONTEND_EVAL_MODEL,
  getFrontendApiKey,
} from "./frontendCommon";
import type { FrontendEvalTransportSettings } from "../core/frontendTransport";

export function createAppModeRunner(
  modelConfig: FrontendEvalModelConfig = DEFAULT_FRONTEND_EVAL_MODEL,
  transportSettings?: FrontendEvalTransportSettings,
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
          provider: modelConfig.provider,
          model: modelConfig.model,
          transport: transportSettings?.transport,
          backend: transportSettings?.backend,
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
      };
    },
    validate({ evalCase, actual, initial, expected }) {
      return validateAppState({
        actual,
        initial,
        expected,
        validate: evalCase.validate as AppValidationSpec | undefined,
      });
    },
    buildArtifacts(actual): BenchmarkArtifactFile[] {
      return buildAppArtifacts(actual);
    },
  };
}
