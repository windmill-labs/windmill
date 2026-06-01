import { readFile } from "node:fs/promises";
import {
  runGlobalEval,
  type GlobalLiveEditorDraftFixture,
} from "../adapters/frontend/core/global/globalEvalRunner";
import type { BenchmarkWorkspaceRunnables } from "../adapters/frontend/mockBackend";
import type { FrontendEvalModelConfig } from "../core/models";
import type { BenchmarkArtifactFile, GlobalValidationSpec, ModeRunner } from "../core/types";
import { validateGlobalState, type GlobalDraftState } from "../core/validators";
import type { WindmillBackendSettings } from "../core/windmillBackendSettings";
import { getFrontendApiKey } from "./frontendCommon";

export interface GlobalInitialFixture {
  workspace?: BenchmarkWorkspaceRunnables;
  liveEditorDrafts?: GlobalLiveEditorDraftFixture[];
}

export function createGlobalModeRunner(
  modelConfig: FrontendEvalModelConfig,
  backendSettings: WindmillBackendSettings,
): ModeRunner<GlobalInitialFixture, GlobalDraftState, GlobalDraftState> {
  return {
    mode: "global",
    concurrency: 3,
    judgeThreshold: 80,
    async loadInitial(path) {
      return path ? await loadGlobalInitialFixture(path) : undefined;
    },
    async loadExpected(path) {
      return path ? await loadGlobalExpectedFixture(path) : undefined;
    },
    async run(prompt, initial, context) {
      const result = await runGlobalEval(
        prompt,
        getFrontendApiKey(modelConfig.provider),
        {
          workspaceFixtures: initial?.workspace,
          liveEditorDrafts: initial?.liveEditorDrafts,
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
    validate({ evalCase, actual, expected }) {
      return validateGlobalState({
        actual,
        expected,
        validate: evalCase.validate as GlobalValidationSpec | undefined,
      });
    },
    buildArtifacts(actual): BenchmarkArtifactFile[] {
      return [
        {
          path: "global-drafts.json",
          content: JSON.stringify(actual, null, 2) + "\n",
        },
      ];
    },
  };
}

async function loadGlobalInitialFixture(path: string): Promise<GlobalInitialFixture> {
  const parsed = JSON.parse(await readFile(path, "utf8")) as GlobalInitialFixture;
  return {
    workspace: parsed.workspace ?? {},
    liveEditorDrafts: parsed.liveEditorDrafts ?? [],
  };
}

async function loadGlobalExpectedFixture(path: string): Promise<GlobalDraftState> {
  return JSON.parse(await readFile(path, "utf8")) as GlobalDraftState;
}
