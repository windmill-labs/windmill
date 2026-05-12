import { mkdtemp, rm } from "fs/promises";
import { tmpdir } from "os";
import { join } from "path";
import type { AIProvider } from "$lib/gen/types.gen";
import {
  globalTools,
  prepareGlobalSystemMessage,
  prepareGlobalUserMessage,
} from "../../../../../frontend/src/lib/components/copilot/chat/global/core";
import { globalDraftStore } from "../../../../../frontend/src/lib/components/copilot/chat/global/draftStore.svelte";
import type { Tool as ProductionTool } from "../../../../../frontend/src/lib/components/copilot/chat/shared";
import type { ModeRunContext } from "../../../../core/types";
import type { GlobalDraftState } from "../../../../core/validators";
import type { WindmillBackendSettings } from "../../../../core/windmillBackendSettings";
import {
  registerBenchmarkWorkspaceRunnables,
  unregisterBenchmarkWorkspaceRunnables,
  type BenchmarkWorkspaceRunnables,
} from "../../mockBackend";
import { runEval } from "../shared";
import type { TokenUsage, ToolCallDetail } from "../shared/types";

const MUTATING_GLOBAL_TOOLS = new Set([
  "deploy_workspace_item",
  "delete_workspace_item",
]);

export interface GlobalEvalResult {
  success: boolean;
  state: GlobalDraftState;
  error?: string;
  assistantMessageCount: number;
  toolCallCount: number;
  toolsUsed: string[];
  toolCallDetails: ToolCallDetail[];
  tokenUsage: TokenUsage;
}

export interface GlobalEvalOptions {
  workspaceFixtures?: BenchmarkWorkspaceRunnables;
  model?: string;
  maxIterations?: number;
  provider?: AIProvider;
  backend: WindmillBackendSettings;
  workspaceRoot?: string;
  runContext?: ModeRunContext;
}

export async function runGlobalEval(
  userPrompt: string,
  apiKey: string,
  options: GlobalEvalOptions,
): Promise<GlobalEvalResult> {
  const workspaceRoot =
    options.workspaceRoot ??
    (await mkdtemp(join(tmpdir(), "wmill-frontend-global-benchmark-")));

  globalDraftStore.clearDrafts(workspaceRoot);
  registerBenchmarkWorkspaceRunnables(workspaceRoot, options.workspaceFixtures ?? {});

  try {
    const model = options.model ?? "claude-haiku-4-5-20251001";
    const rawResult = await runEval({
      userPrompt,
      systemMessage: prepareGlobalSystemMessage(),
      userMessage: prepareGlobalUserMessage(userPrompt),
      tools: getGlobalEvalTools(),
      helpers: {},
      apiKey,
      getOutput: () => ({ drafts: globalDraftStore.listDrafts(workspaceRoot) }),
      onAssistantMessageStart: options.runContext?.onAssistantMessageStart,
      onAssistantToken: options.runContext?.onAssistantChunk,
      onAssistantMessageEnd: options.runContext?.onAssistantMessageEnd,
      onToolCall: options.runContext?.onToolCall,
      options: {
        maxIterations: options.maxIterations,
        model,
        workspace: workspaceRoot,
        provider: options.provider,
        backend: options.backend,
        caseId: options.runContext?.caseId,
        attempt: options.runContext?.attempt,
      },
    });

    return {
      state: rawResult.output,
      success: rawResult.success,
      error: rawResult.error,
      assistantMessageCount: rawResult.iterations,
      toolCallCount: rawResult.toolCallsCount,
      toolsUsed: rawResult.toolsCalled,
      toolCallDetails: rawResult.toolCallDetails,
      tokenUsage: rawResult.tokenUsage,
    };
  } finally {
    globalDraftStore.clearDrafts(workspaceRoot);
    unregisterBenchmarkWorkspaceRunnables(workspaceRoot);
    if (!options.workspaceRoot) {
      await rm(workspaceRoot, { recursive: true, force: true });
    }
  }
}

function getGlobalEvalTools(): ProductionTool<{}>[] {
  return (globalTools as ProductionTool<{}>[]).map((tool) => {
    if (!MUTATING_GLOBAL_TOOLS.has(tool.def.function.name)) {
      return tool;
    }

    return {
      ...tool,
      requiresConfirmation: false,
      validateBeforeConfirmation: undefined,
      fn: async () =>
        JSON.stringify(
          {
            success: false,
            error:
              "This mutating workspace tool is disabled during ai_evals global mode.",
          },
          null,
          2,
        ),
    };
  });
}
