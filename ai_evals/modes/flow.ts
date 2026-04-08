import { readJsonFile } from "../core/files";
import { validateFlowState, type FlowState } from "../core/validators";
import type { ModeRunner } from "../core/types";
import {
  runFlowEval,
  type FlowFixture,
} from "../adapters/frontend/core/flow/flowEvalRunner";
import type { FlowWorkspaceFixtures } from "../adapters/frontend/core/flow/fileHelpers";
import { FRONTEND_MODEL, FRONTEND_PROVIDER, getFrontendApiKey } from "./frontendCommon";

interface FlowInitialFixture {
  flow?: FlowFixture;
  workspace?: FlowWorkspaceFixtures;
}

export function createFlowModeRunner(): ModeRunner<FlowInitialFixture, FlowState, FlowState> {
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
      const result = await runFlowEval(prompt, getFrontendApiKey(), {
        initialFlow: initial?.flow,
        workspaceFixtures: initial?.workspace,
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
    validate({ actual, initial, expected }) {
      return validateFlowState({ actual, initial: initial?.flow, expected });
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
