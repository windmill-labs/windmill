import { readJsonFile } from "../core/files";
import type { FrontendEvalModelConfig } from "../core/models";
import { validateFlowState, type FlowState } from "../core/validators";
import type { BenchmarkArtifactFile, ModeRunner } from "../core/types";
import {
  runFlowEval,
  type FlowFixture,
} from "../adapters/frontend/core/flow/flowEvalRunner";
import type { FlowWorkspaceFixtures } from "../adapters/frontend/core/flow/fileHelpers";
import { DEFAULT_FRONTEND_EVAL_MODEL, getFrontendApiKey } from "./frontendCommon";

interface FlowInitialFixture {
  flow?: FlowFixture;
  workspace?: FlowWorkspaceFixtures;
}

export function createFlowModeRunner(
  modelConfig: FrontendEvalModelConfig = DEFAULT_FRONTEND_EVAL_MODEL
): ModeRunner<FlowInitialFixture, FlowState, FlowState> {
  return {
    mode: "flow",
    concurrency: 5,
    judgeThreshold: 80,
    async loadInitial(path) {
      if (!path) {
        return undefined;
      }
      return normalizeFlowInitialFixture(await readJsonFile<unknown>(path));
    },
    async loadExpected(path) {
      if (!path) {
        return undefined;
      }
      return normalizeFlowStateFixture(await readJsonFile<unknown>(path));
    },
    async run(prompt, initial, context) {
      const result = await runFlowEval(prompt, getFrontendApiKey(modelConfig.provider), {
        initialFlow: initial?.flow,
        workspaceFixtures: initial?.workspace,
        provider: modelConfig.provider,
        model: modelConfig.model,
        runContext: context,
      });

      return {
        success: result.success,
        actual: normalizeFlowStateFixture(result.flow),
        error: result.error,
        assistantMessageCount: result.assistantMessageCount,
        toolCallCount: result.toolCallCount,
        toolsUsed: result.toolsUsed,
        skillsInvoked: [],
        tokenUsage: result.tokenUsage,
      };
    },
    validate({ evalCase, actual, initial, expected }) {
      return validateFlowState({
        actual,
        initial: initial?.flow,
        expected,
        validate: evalCase.validate,
      });
    },
    buildArtifacts(actual): BenchmarkArtifactFile[] {
      return [
        {
          path: "flow.json",
          content: JSON.stringify(actual, null, 2) + "\n",
        },
      ];
    },
  };
}

function normalizeFlowInitialFixture(value: unknown): FlowInitialFixture {
  if (isObject(value) && ("flow" in value || "workspace" in value)) {
    const fixture = value as {
      flow?: FlowFixture;
      workspace?: FlowWorkspaceFixtures;
    };
    return {
      flow: fixture.flow,
      workspace: fixture.workspace,
    };
  }

  return {
    flow: normalizeFlowStateFixture(value),
  };
}

function normalizeFlowStateFixture(value: unknown): FlowState {
  if (!isObject(value)) {
    return {};
  }
  if ("flow" in value && isObject((value as { flow?: unknown }).flow)) {
    return (value as { flow: FlowState }).flow;
  }
  return value as FlowState;
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
