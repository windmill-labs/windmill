import {
  runAskEval,
  resolveDocsToolVariant,
} from "../adapters/frontend/core/ask/askEvalRunner";
import type { FrontendEvalModelConfig } from "../core/models";
import type {
  AskValidationSpec,
  BenchmarkArtifactFile,
  ModeRunner,
} from "../core/types";
import { validateAskAnswer, type AskAnswerState } from "../core/validators";
import type { WindmillBackendSettings } from "../core/windmillBackendSettings";
import { getFrontendApiKey } from "./frontendCommon";

export function createAskModeRunner(
  modelConfig: FrontendEvalModelConfig,
  backendSettings: WindmillBackendSettings,
): ModeRunner<undefined, undefined, AskAnswerState> {
  const variant = resolveDocsToolVariant();

  return {
    mode: "ask",
    concurrency: 2,
    judgeThreshold: 80,
    async loadInitial() {
      return undefined;
    },
    async loadExpected() {
      return undefined;
    },
    async run(prompt, _initial, context) {
      const result = await runAskEval(
        prompt,
        getFrontendApiKey(modelConfig.provider),
        {
          variant,
          maxIterations: context.evalCase?.runtime?.maxTurns,
          provider: modelConfig.provider,
          model: modelConfig.model,
          backend: backendSettings,
          runContext: context,
        },
      );

      return {
        success: result.success,
        actual: result.state,
        error: result.error,
        assistantMessageCount: result.assistantMessageCount,
        toolCallCount: result.toolCallCount,
        toolsUsed: result.toolsUsed,
        toolCallDetails: result.toolCallDetails,
        skillsInvoked: [],
        tokenUsage: result.tokenUsage,
      };
    },
    validate({ evalCase, actual }) {
      return validateAskAnswer({
        actual,
        validate: evalCase.validate as AskValidationSpec | undefined,
      });
    },
    // The judge must stay blind to which docs-tool arm produced the answer, so
    // it only ever sees the answer text.
    prepareJudgeActual(actual) {
      return { answer: actual.answer };
    },
    buildArtifacts(actual): BenchmarkArtifactFile[] {
      return [
        {
          path: "ask-answer.md",
          content: `${actual.answer}\n`,
        },
        {
          path: "ask-meta.json",
          content:
            JSON.stringify(
              {
                docsTool: actual.docsTool,
                toolsUsed: [...new Set(actual.toolsUsed)],
                toolCallCount: actual.toolCallCount,
              },
              null,
              2,
            ) + "\n",
        },
      ];
    },
  };
}
