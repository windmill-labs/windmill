import { readJsonFile } from "../core/files";
import type { BackendValidationSettings } from "../core/backendValidation";
import type { FrontendEvalModelConfig } from "../core/models";
import { validateScriptState } from "../core/validators";
import type { BenchmarkArtifactFile, ModeRunner } from "../core/types";
import { BackendPreviewClient } from "../adapters/frontend/backendPreview";
import { runScriptEval } from "../adapters/frontend/core/script/scriptEvalRunner";
import type { ScriptEvalState } from "../adapters/frontend/core/script/fileHelpers";
import { DEFAULT_FRONTEND_EVAL_MODEL, getFrontendApiKey } from "./frontendCommon";

export function createScriptModeRunner(
  modelConfig: FrontendEvalModelConfig = DEFAULT_FRONTEND_EVAL_MODEL,
  backendValidation?: BackendValidationSettings
): ModeRunner<ScriptEvalState, ScriptEvalState, ScriptEvalState> {
  return {
    mode: "script",
    concurrency: 5,
    judgeThreshold: 80,
    async loadInitial(path) {
      return path ? await readJsonFile<ScriptEvalState>(path) : undefined;
    },
    async loadExpected(path) {
      return path ? await readJsonFile<ScriptEvalState>(path) : undefined;
    },
    async run(prompt, initial, context) {
      if (!initial) {
        throw new Error("Script evals require an initial script fixture");
      }

      const result = await runScriptEval(prompt, getFrontendApiKey(modelConfig.provider), {
        initialScript: initial,
        maxIterations: context.evalCase?.runtime?.maxTurns,
        provider: modelConfig.provider,
        model: modelConfig.model,
        runContext: context,
      });

      return {
        success: result.success,
        actual: result.script,
        error: result.error,
        assistantMessageCount: result.assistantMessageCount,
        toolCallCount: result.toolCallCount,
        toolsUsed: result.toolsUsed,
        skillsInvoked: [],
        tokenUsage: result.tokenUsage,
      };
    },
    validate({ actual, initial, expected }) {
      return validateScriptState({ actual, initial, expected });
    },
    async backendValidate({ evalCase, initial, actual, context }) {
      if (backendValidation?.mode !== "preview") {
        return null;
      }

      const previewClient = new BackendPreviewClient(backendValidation);
      return await previewClient.withWorkspace(evalCase.id, context.attempt, async (workspaceId) => {
        const completedJob = await previewClient.runScriptPreview({
          workspaceId,
          content: actual.code,
          args:
            (evalCase.runtime?.backendPreview?.args as Record<string, unknown> | undefined) ??
            actual.args ??
            initial?.args ??
            {},
          language: normalizePreviewLanguage(actual.lang),
          path: toPreviewScriptPath(actual.path),
          timeoutSeconds: evalCase.runtime?.backendPreview?.timeoutSeconds,
        });

        return {
          checks: [
            {
              name: "backend script preview succeeded",
              passed: completedJob.success,
              details: completedJob.success
                ? `workspace=${workspaceId}`
                : `workspace=${workspaceId}; job=${completedJob.id}`,
            },
          ],
          artifactFiles: [
            {
              path: "backend-preview.json",
              content:
                JSON.stringify(
                  {
                    workspaceId,
                    jobId: completedJob.id,
                    success: completedJob.success,
                    result: completedJob.result,
                    logs: completedJob.logs,
                    completedJob: completedJob.raw,
                  },
                  null,
                  2
                ) + "\n",
            },
          ],
        };
      });
    },
    buildArtifacts(actual): BenchmarkArtifactFile[] {
      return [
        {
          path: "script.json",
          content: JSON.stringify(actual, null, 2) + "\n",
        },
        {
          path: actual.path,
          content: actual.code,
        },
      ];
    },
  };
}

function normalizePreviewLanguage(language: ScriptEvalState["lang"]): string {
  return language === "bunnative" ? "bun" : language;
}

function toPreviewScriptPath(filePath: string): string {
  return filePath.replace(/\.[^.\/]+$/, "");
}
