import { readJsonFile } from "../core/files";
import { validateFlowState, type FlowState } from "../core/validators";
import type { ModeRunner } from "../core/types";
import { runFlowEval } from "../adapters/frontend/core/flow/flowEvalRunner";
import { FRONTEND_MODEL, FRONTEND_PROVIDER, getFrontendApiKey } from "./frontendCommon";

export function createFlowModeRunner(): ModeRunner<FlowState, FlowState, FlowState> {
  return {
    mode: "flow",
    concurrency: 5,
    judgeThreshold: 80,
    async loadInitial(path) {
      return path ? await readJsonFile<FlowState>(path) : undefined;
    },
    async loadExpected(path) {
      return path ? await readJsonFile<FlowState>(path) : undefined;
    },
    async run(prompt, initial, context) {
      const result = await runFlowEval(prompt, getFrontendApiKey(), {
        initialFlow: initial,
        provider: FRONTEND_PROVIDER,
        model: FRONTEND_MODEL,
        runContext: context,
      });

      return {
        success: result.success,
        actual: {
          value: { modules: result.flow.value?.modules ?? [] },
          schema: result.flow.schema,
        },
        error: result.error,
        assistantMessageCount: result.assistantMessageCount,
        toolCallCount: result.toolCallCount,
        toolsUsed: result.toolsUsed,
        skillsInvoked: [],
      };
    },
    validate({ actual, expected }) {
      return validateFlowState({ actual, expected });
    },
  };
}
