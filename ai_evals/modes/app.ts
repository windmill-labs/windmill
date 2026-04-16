import { loadAppFixture } from "../adapters/frontend/core/app/appFixtureLoader";
import type { AppFiles } from "../../frontend/src/lib/components/copilot/chat/app/core";
import type { FrontendEvalModelConfig } from "../core/models";
import { validateAppState, type AppFilesState } from "../core/validators";
import type { BenchmarkArtifactFile, ModeRunner } from "../core/types";
import { runAppEval } from "../adapters/frontend/core/app/appEvalRunner";
import { DEFAULT_FRONTEND_EVAL_MODEL, getFrontendApiKey } from "./frontendCommon";

export function createAppModeRunner(
  modelConfig: FrontendEvalModelConfig = DEFAULT_FRONTEND_EVAL_MODEL
): ModeRunner<AppFilesState, AppFilesState, AppFilesState> {
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
    async run(prompt, initial, context) {
      const result = await runAppEval(prompt, getFrontendApiKey(modelConfig.provider), {
        initialFrontend: initial?.frontend,
        initialBackend: initial?.backend as AppFiles["backend"] | undefined,
        maxIterations: context.evalCase?.runtime?.maxTurns,
        provider: modelConfig.provider,
        model: modelConfig.model,
        runContext: context,
      });

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
    validate({ actual, initial, expected }) {
      return validateAppState({ actual, initial, expected });
    },
    buildArtifacts(actual): BenchmarkArtifactFile[] {
      const artifacts: BenchmarkArtifactFile[] = [
        {
          path: "app.json",
          content: JSON.stringify(actual, null, 2) + "\n",
        },
      ];

      for (const [filePath, content] of Object.entries(actual.frontend)) {
        artifacts.push({
          path: `frontend${filePath.startsWith("/") ? filePath : `/${filePath}`}`,
          content,
        });
      }

      for (const [key, runnable] of Object.entries(actual.backend)) {
        artifacts.push({
          path: `backend/${key}/meta.json`,
          content: JSON.stringify(runnable, null, 2) + "\n",
        });

        const inlineContent = runnable.inlineScript?.content;
        if (inlineContent) {
          const extension = runnable.inlineScript?.language === "python3" ? "py" : "ts";
          artifacts.push({
            path: `backend/${key}/main.${extension}`,
            content: inlineContent,
          });
        }
      }

      return artifacts;
    },
  };
}
