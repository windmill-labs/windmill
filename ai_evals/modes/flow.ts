import { readJsonFile } from "../core/files";
import type { BackendValidationSettings } from "../core/backendValidation";
import type { FrontendEvalModelConfig } from "../core/models";
import type { FlowValidationSpec } from "../core/types";
import { validateFlowState, type FlowState } from "../core/validators";
import type { BenchmarkArtifactFile, ModeRunner } from "../core/types";
import { runFlowEval } from "../adapters/frontend/core/flow/flowEvalRunner";
import type { FlowWorkspaceFixtures } from "../adapters/frontend/core/flow/fileHelpers";
import { BackendPreviewClient } from "../adapters/frontend/backendPreview";
import {
  DEFAULT_FRONTEND_EVAL_MODEL,
  getFrontendApiKey,
} from "./frontendCommon";
import type { FrontendEvalTransportSettings } from "../core/frontendTransport";
import {
  normalizeFlowInitialFixture,
  normalizeFlowStateFixture,
  type FlowInitialFixture,
} from "./flowFixtures";

export function createFlowModeRunner(
  modelConfig: FrontendEvalModelConfig = DEFAULT_FRONTEND_EVAL_MODEL,
  backendValidation?: BackendValidationSettings,
  transportSettings?: FrontendEvalTransportSettings,
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
      const result = await runFlowEval(
        prompt,
        getFrontendApiKey(modelConfig.provider),
        {
          initialFlow: initial?.flowFixture,
          workspaceFixtures: initial?.workspace,
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
        initial: initial?.flowState,
        expected,
        validate: evalCase.validate as FlowValidationSpec | undefined,
      });
    },
    async backendValidate({ evalCase, initial, actual, context }) {
      if (
        backendValidation?.mode !== "preview" ||
        !evalCase.runtime?.backendPreview
      ) {
        return null;
      }

      if (!actual.value) {
        return {
          checks: [
            {
              name: "backend flow preview succeeded",
              passed: false,
              details: "Generated flow is missing value.modules",
            },
          ],
        };
      }

      const previewClient = new BackendPreviewClient(backendValidation);
      return await previewClient.withWorkspace(
        evalCase.id,
        context.attempt,
        async (workspaceId) => {
          await seedWorkspaceFixtures(
            previewClient,
            workspaceId,
            initial?.workspace,
          );

          const completedJob = await previewClient.runFlowPreview({
            workspaceId,
            value: actual.value as Record<string, unknown>,
            args: evalCase.runtime?.backendPreview?.args ?? {},
            timeoutSeconds: evalCase.runtime?.backendPreview?.timeoutSeconds,
          });

          return {
            checks: [
              {
                name: "backend flow preview succeeded",
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
                    2,
                  ) + "\n",
              },
            ],
          };
        },
      );
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

async function seedWorkspaceFixtures(
  previewClient: BackendPreviewClient,
  workspaceId: string,
  fixtures?: FlowWorkspaceFixtures,
): Promise<void> {
  for (const script of fixtures?.scripts ?? []) {
    await previewClient.createScript({
      workspaceId,
      path: script.path,
      summary: script.summary,
      description: script.description,
      schema: script.schema,
      content: script.content,
      language: script.language,
    });
  }

  for (const flow of fixtures?.flows ?? []) {
    await previewClient.createFlow({
      workspaceId,
      path: flow.path,
      summary: flow.summary,
      description: flow.description,
      schema: flow.schema,
      value: flow.value as Record<string, unknown>,
    });
  }
}
