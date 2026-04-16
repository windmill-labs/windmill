import type { FlowModule } from "../../frontend/src/lib/gen";
import type { FlowFixture } from "../adapters/frontend/core/flow/flowEvalRunner";
import type { FlowWorkspaceFixtures } from "../adapters/frontend/core/flow/fileHelpers";
import type { FlowState } from "../core/validators";

export interface FlowInitialFixture {
  flowFixture?: FlowFixture;
  flowState?: FlowState;
  workspace?: FlowWorkspaceFixtures;
}

export function normalizeFlowInitialFixture(value: unknown): FlowInitialFixture {
  if (isObject(value) && ("flow" in value || "workspace" in value)) {
    const fixture = value as {
      flow?: unknown;
      workspace?: FlowWorkspaceFixtures;
    };
    const flowState = fixture.flow ? normalizeFlowStateFixture(fixture.flow) : undefined;
    return {
      flowState,
      flowFixture: flowState ? normalizeFlowFixture(flowState) : undefined,
      workspace: fixture.workspace,
    };
  }

  const flowState = normalizeFlowStateFixture(value);
  return {
    flowState,
    flowFixture: normalizeFlowFixture(flowState),
  };
}

export function normalizeFlowStateFixture(value: unknown): FlowState {
  if (!isObject(value)) {
    return {};
  }
  if ("flow" in value && isObject((value as { flow?: unknown }).flow)) {
    return (value as { flow: FlowState }).flow;
  }
  return value as FlowState;
}

export function normalizeFlowFixture(value: FlowState): FlowFixture {
  return {
    schema: value.schema,
    value: value.value
      ? {
          modules: value.value.modules as FlowModule[] | undefined,
          preprocessor_module: value.value.preprocessor_module as FlowModule | undefined,
          failure_module: value.value.failure_module as FlowModule | undefined,
        }
      : undefined,
  };
}

function isObject(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
