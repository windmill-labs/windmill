import { loadAppFixture } from "../adapters/frontend/core/app/appFixtureLoader";
import type { AppFiles } from "../../frontend/src/lib/components/copilot/chat/app/core";
import { validateAppState, type AppFilesState } from "../core/validators";
import type { ModeRunner } from "../core/types";
import { runAppEval } from "../adapters/frontend/core/app/appEvalRunner";
import { FRONTEND_MODEL, FRONTEND_PROVIDER, getFrontendApiKey } from "./frontendCommon";

export function createAppModeRunner(): ModeRunner<AppFilesState, AppFilesState, AppFilesState> {
  return {
    mode: "app",
    concurrency: 5,
    judgeThreshold: 80,
    async loadInitial(path) {
      return path ? (await loadAppFixture(path)) : undefined;
    },
    async loadExpected(path) {
      return path ? (await loadAppFixture(path)) : undefined;
    },
    async run(prompt, initial) {
      const result = await runAppEval(prompt, getFrontendApiKey(), {
        initialFrontend: initial?.frontend,
        initialBackend: initial?.backend as AppFiles["backend"] | undefined,
        provider: FRONTEND_PROVIDER,
        model: FRONTEND_MODEL,
      });

      return {
        success: result.success,
        actual: result.files as AppFilesState,
        error: result.error,
        assistantMessageCount: result.assistantMessageCount,
        toolCallCount: result.toolCallCount,
        toolsUsed: result.toolsUsed,
        skillsInvoked: [],
      };
    },
    validate({ actual, initial, expected }) {
      return validateAppState({ actual, initial, expected });
    },
  };
}
