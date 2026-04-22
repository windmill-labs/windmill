import type {
  ChatCompletionMessageParam,
  ChatCompletionSystemMessageParam,
} from "openai/resources/chat/completions.mjs";
import type { AIProvider } from "$lib/gen/types.gen";
import type { ToolCallDetail, EvalRunnerOptions, RawEvalResult } from "./types";
import {
  runChatLoop,
  type ChatClients,
} from "../../../../../frontend/src/lib/components/copilot/chat/chatLoop";
import type {
  Tool as ProductionTool,
  ToolCallbacks,
} from "../../../../../frontend/src/lib/components/copilot/chat/shared";
import {
  buildProxyResourcePath,
  createEvalClients,
  type FrontendEvalProvider,
  resolveEvalModelProvider,
} from "./providerConfig";
import { WindmillBackendClient } from "../../windmillBackend";

/**
 * Parameters for running a base evaluation.
 */
export interface RunEvalParams<THelpers, TOutput> {
  /** The user's prompt/instruction */
  userPrompt: string;
  /** System message for the LLM */
  systemMessage: ChatCompletionSystemMessageParam;
  /** User message for the LLM */
  userMessage: ChatCompletionMessageParam;
  /** Tool definitions for the LLM API (unused — derived from tools) */
  toolDefs?: unknown;
  /** Full tool implementations for execution */
  tools: ProductionTool<THelpers>[];
  /** Domain-specific helpers for tool execution */
  helpers: THelpers;
  /** API key for the provider */
  apiKey: string;
  /** Function to get the current output state */
  getOutput: () => TOutput;
  /** Optional configuration */
  options?: EvalRunnerOptions;
  onAssistantMessageStart?: () => void;
  onAssistantToken?: (token: string) => void;
  onAssistantMessageEnd?: () => void;
  onToolCall?: (input: { toolName: string; argumentsText: string }) => void;
}

/**
 * Runs a generic evaluation using the shared chat loop (same code path as production).
 * Uses streaming via real provider SDKs instead of OpenRouter non-streaming.
 */
export async function runEval<THelpers, TOutput>(
  params: RunEvalParams<THelpers, TOutput>,
): Promise<RawEvalResult<TOutput>> {
  const {
    systemMessage,
    userMessage,
    tools,
    helpers,
    apiKey,
    getOutput,
    options,
    onAssistantMessageStart,
    onAssistantToken,
    onAssistantMessageEnd,
    onToolCall,
  } = params;
  let shouldEmitMessageStart = true;

  const model = options?.model ?? "gpt-4o";
  const maxIterations = options?.maxIterations ?? 20;
  const workspace = options?.workspace ?? "test-workspace";
  const provider = toFrontendEvalProvider(options?.provider);

  const modelProvider = resolveEvalModelProvider(model, provider);

  const messages: ChatCompletionMessageParam[] = [userMessage];
  let toolCallsCount = 0;
  const toolsCalled: string[] = [];
  const toolCallDetails: ToolCallDetail[] = [];

  // Wrap tools to intercept fn calls for tracking.
  // Cast to ProductionTool since the eval Tool has a narrower toolCallbacks type
  // but the actual callbacks passed at runtime will satisfy both interfaces.
  const wrappedTools = tools.map((tool) => ({
    ...tool,
    fn: async (p: any) => {
      toolCallsCount++;
      toolsCalled.push(tool.def.function.name);
      let argumentsText = "";
      try {
        const args = typeof p.args === "string" ? JSON.parse(p.args) : p.args;
        toolCallDetails.push({ name: tool.def.function.name, arguments: args });
        argumentsText = JSON.stringify(args);
      } catch {
        toolCallDetails.push({
          name: tool.def.function.name,
          arguments: p.args,
        });
        argumentsText =
          typeof p.args === "string" ? p.args : JSON.stringify(p.args);
      }
      onToolCall?.({
        toolName: tool.def.function.name,
        argumentsText,
      });
      return tool.fn(p);
    },
  }));

  // No-op callbacks for eval
  const callbacks: ToolCallbacks & {
    onNewToken: (token: string) => void;
    onMessageEnd: () => void;
  } = {
    setToolStatus: () => {},
    removeToolStatus: () => {},
    onNewToken: (token: string) => {
      if (shouldEmitMessageStart) {
        onAssistantMessageStart?.();
        shouldEmitMessageStart = false;
      }
      onAssistantToken?.(token);
    },
    onMessageEnd: () => {
      if (!shouldEmitMessageStart) {
        onAssistantMessageEnd?.();
      }
      shouldEmitMessageStart = true;
    },
  };

  const abortController = new AbortController();

  const executeChatLoop = async (clients: ChatClients) => {
    try {
      const result = await runChatLoop({
        messages,
        systemMessage,
        tools: wrappedTools,
        helpers,
        abortController,
        callbacks,
        modelProvider,
        clients,
        workspace,
        maxIterations,
        skipResponsesApi: modelProvider.provider !== "openai",
      });

      if (result.hitMaxIterations) {
        return {
          success: false,
          output: getOutput(),
          error: `Reached max turns (${maxIterations})`,
          tokenUsage: result.tokenUsage,
          toolCallsCount,
          toolsCalled,
          toolCallDetails,
          iterations: Math.max(
            1,
            result.addedMessages.filter((m) => m.role === "assistant").length,
          ),
          messages,
        };
      }

      return {
        success: true,
        output: getOutput(),
        tokenUsage: result.tokenUsage,
        toolCallsCount,
        toolsCalled,
        toolCallDetails,
        iterations: Math.max(
          1,
          result.addedMessages.filter((m) => m.role === "assistant").length,
        ),
        messages,
      };
    } catch (err) {
      let errorMessage: string;
      if (err instanceof Error) {
        errorMessage = err.stack ?? err.message;
      } else {
        errorMessage = String(err);
      }

      return {
        success: false,
        output: getOutput(),
        error: errorMessage,
        tokenUsage: { prompt: 0, completion: 0, total: 0 },
        toolCallsCount,
        toolsCalled,
        toolCallDetails,
        iterations: 0,
        messages,
      };
    }
  };

  if (options?.transport === "proxy") {
    const backendSettings = options.backend;
    if (!backendSettings) {
      throw new Error("Missing backend settings for proxy transport");
    }

    const backendClient = new WindmillBackendClient(backendSettings);
    return await backendClient.withWorkspace(
      options.proxyCaseId ?? "eval",
      options.proxyAttempt ?? 1,
      async (proxyWorkspaceId) => {
        const resourcePath = buildProxyResourcePath(modelProvider.provider);
        await backendClient.upsertResource({
          workspaceId: proxyWorkspaceId,
          path: resourcePath,
          resourceType: modelProvider.provider,
          value: { api_key: apiKey },
        });
        const token = await backendClient.getToken();
        const clients = createEvalClients({
          provider: modelProvider.provider,
          apiKey,
          transport: "proxy",
          proxy: {
            baseURL: `${backendSettings.baseUrl}/api/w/${encodeURIComponent(proxyWorkspaceId)}/ai/proxy`,
            bearerToken: token,
            resourcePath,
          },
        }) as unknown as ChatClients;
        return await executeChatLoop(clients);
      },
    );
  }

  const clients = createEvalClients({
    provider: modelProvider.provider,
    apiKey,
  }) as unknown as ChatClients;
  return await executeChatLoop(clients);
}

function toFrontendEvalProvider(
  provider?: AIProvider,
): FrontendEvalProvider | undefined {
  if (
    provider === "anthropic" ||
    provider === "openai" ||
    provider === "googleai"
  ) {
    return provider;
  }
  return undefined;
}
