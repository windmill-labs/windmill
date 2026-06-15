import type { ChatCompletionMessageParam } from "openai/resources/chat/completions.mjs";
import type { AIProvider } from "$lib/gen/types.gen";
import {
  getAskTools,
  prepareAskSystemMessage,
  prepareAskUserMessage,
  type DocsToolVariant,
} from "../../../../../frontend/src/lib/components/copilot/chat/ask/core";
import type { ModeRunContext } from "../../../../core/types";
import type { AskAnswerState } from "../../../../core/validators";
import type { WindmillBackendSettings } from "../../../../core/windmillBackendSettings";
import { WindmillBackendClient } from "../../windmillBackend";
import { runEval } from "../shared";
import type { TokenUsage, ToolCallDetail } from "../shared/types";

export interface AskEvalResult {
  success: boolean;
  state: AskAnswerState;
  error?: string;
  assistantMessageCount: number;
  toolCallCount: number;
  toolsUsed: string[];
  toolCallDetails: ToolCallDetail[];
  tokenUsage: TokenUsage;
}

export interface AskEvalOptions {
  variant: DocsToolVariant;
  model?: string;
  maxIterations?: number;
  provider?: AIProvider;
  backend: WindmillBackendSettings;
  runContext?: ModeRunContext;
}

const DOCS_TOOL_ENV = "WMILL_AI_EVAL_DOCS_TOOL";

/**
 * Resolves which docs-tool arm to benchmark. Defaults to the new llms.txt arm.
 */
export function resolveDocsToolVariant(
  value: string | undefined = process.env[DOCS_TOOL_ENV],
): DocsToolVariant {
  return value === "inkeep" ? "inkeep" : "llmstxt";
}

export async function runAskEval(
  userPrompt: string,
  apiKey: string,
  options: AskEvalOptions,
): Promise<AskEvalResult> {
  // The production inkeep tool calls fetch('/api/inkeep') with a relative URL,
  // which has no origin under node. Install a process-wide shim that rewrites
  // /api/* calls to the eval backend with an auth token. Only needed for the
  // inkeep arm; the llms.txt arm fetches absolute windmill.dev URLs natively.
  if (options.variant === "inkeep") {
    await installInkeepFetchShim(options.backend);
  }

  const model = options.model ?? "claude-haiku-4-5-20251001";

  const rawResult = await runEval({
    userPrompt,
    systemMessage: prepareAskSystemMessage(undefined, options.variant),
    userMessage: prepareAskUserMessage(userPrompt),
    tools: getAskTools(options.variant),
    helpers: {},
    apiKey,
    getOutput: () => ({
      answer: "",
      docsTool: options.variant,
      toolsUsed: [],
      toolCallCount: 0,
    }),
    onAssistantMessageStart: options.runContext?.onAssistantMessageStart,
    onAssistantToken: options.runContext?.onAssistantChunk,
    onAssistantMessageEnd: options.runContext?.onAssistantMessageEnd,
    onToolCall: options.runContext?.onToolCall,
    options: {
      maxIterations: options.maxIterations,
      model,
      provider: options.provider,
      backend: options.backend,
      caseId: options.runContext?.caseId,
      attempt: options.runContext?.attempt,
    },
  });

  const answer = extractFinalAnswer(rawResult.messages);

  return {
    success: rawResult.success,
    state: {
      answer,
      docsTool: options.variant,
      toolsUsed: rawResult.toolsCalled,
      toolCallCount: rawResult.toolCallsCount,
    },
    error: rawResult.error,
    assistantMessageCount: rawResult.iterations,
    toolCallCount: rawResult.toolCallsCount,
    toolsUsed: rawResult.toolsCalled,
    toolCallDetails: rawResult.toolCallDetails,
    tokenUsage: rawResult.tokenUsage,
  };
}

/**
 * The final answer is the last assistant message with non-empty string content.
 */
export function extractFinalAnswer(
  messages: ChatCompletionMessageParam[],
): string {
  for (let i = messages.length - 1; i >= 0; i--) {
    const message = messages[i];
    if (message.role !== "assistant") {
      continue;
    }
    const content = message.content;
    if (typeof content === "string" && content.trim().length > 0) {
      return content;
    }
    if (Array.isArray(content)) {
      const text = content
        .map((part) =>
          part && typeof part === "object" && "text" in part
            ? String((part as { text?: unknown }).text ?? "")
            : "",
        )
        .join("")
        .trim();
      if (text.length > 0) {
        return text;
      }
    }
  }
  return "";
}

let inkeepFetchShimInstalled = false;

/**
 * Installs (once) a process-wide fetch wrapper that rewrites relative /api/*
 * requests to the eval backend, adding a bearer token. All other URLs are
 * passed through untouched.
 */
async function installInkeepFetchShim(
  backend: WindmillBackendSettings,
): Promise<void> {
  if (inkeepFetchShimInstalled) {
    return;
  }
  inkeepFetchShimInstalled = true;

  const client = new WindmillBackendClient(backend);
  const originalFetch = globalThis.fetch.bind(globalThis);

  globalThis.fetch = (async (
    input: RequestInfo | URL,
    init?: RequestInit,
  ): Promise<Response> => {
    if (typeof input === "string" && input.startsWith("/api/")) {
      const token = await client.getToken();
      const url = `${backend.baseUrl}${input}`;
      return await originalFetch(url, {
        ...init,
        headers: {
          ...(init?.headers ?? {}),
          Authorization: `Bearer ${token}`,
        },
      });
    }
    return await originalFetch(input as any, init);
  }) as typeof fetch;
}
