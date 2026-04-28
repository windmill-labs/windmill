import { mkdtemp } from "fs/promises";
import { tmpdir } from "os";
import { join } from "path";
import type {
  BackendRunnable,
  AppAIChatHelpers,
  DataTableSchema,
} from "../../../../../frontend/src/lib/components/copilot/chat/app/core";
import {
  getAppTools,
  prepareAppSystemMessage,
  prepareAppUserMessage,
} from "../../../../../frontend/src/lib/components/copilot/chat/app/core";
import type { Tool as ProductionTool } from "../../../../../frontend/src/lib/components/copilot/chat/shared";
import { createAppFileHelpers } from "./fileHelpers";
import { runEval } from "../shared";
import type { AIProvider } from "$lib/gen/types.gen";
import type {
  EvalCaseRuntimeAppAdditionalContext,
  EvalCaseRuntimeAppContextSpec,
  ModeRunContext,
} from "../../../../core/types";
import type { TokenUsage } from "../shared/types";
import type { AppFilesState } from "../../../../core/validators";
import type { FrontendEvalTransport } from "../../../../core/frontendTransport";
import type { WindmillBackendSettings } from "../../../../core/windmillBackendSettings";
import {
  createAppBackendRunnableContextElement,
  createAppDatatableContextElement,
  createAppFrontendFileContextElement,
  type ContextElement,
} from "../../../../../frontend/src/lib/components/copilot/chat/context";

export interface AppEvalResult {
  success: boolean;
  files: AppFilesState;
  error?: string;
  assistantMessageCount: number;
  toolCallCount: number;
  toolsUsed: string[];
  tokenUsage: TokenUsage;
}

export interface AppEvalOptions {
  initialFrontend?: Record<string, string>;
  initialBackend?: AppFilesState["backend"];
  initialDatatables?: AppFilesState["datatables"];
  appContext?: EvalCaseRuntimeAppContextSpec;
  model?: string;
  maxIterations?: number;
  provider?: AIProvider;
  transport?: FrontendEvalTransport;
  backend?: WindmillBackendSettings;
  workspaceRoot?: string;
  runContext?: ModeRunContext;
}

export async function runAppEval(
  userPrompt: string,
  apiKey: string,
  options?: AppEvalOptions,
): Promise<AppEvalResult> {
  const workspaceRoot =
    options?.workspaceRoot ??
    (await mkdtemp(join(tmpdir(), "wmill-frontend-app-benchmark-")));
  const { helpers, getEvalState, cleanup } = await createAppFileHelpers(
    options?.initialFrontend ?? {},
    (options?.initialBackend ?? {}) as Record<string, BackendRunnable>,
    options?.initialDatatables ?? [],
    workspaceRoot,
  );

  try {
    const systemMessage = prepareAppSystemMessage();
    const tools = getAppTools() as ProductionTool<AppAIChatHelpers>[];
    const model = options?.model ?? "claude-haiku-4-5-20251001";
    const additionalContext = await buildAdditionalContext(
      options?.appContext,
      helpers,
    );
    const userMessage = prepareAppUserMessage(
      userPrompt,
      helpers.getSelectedContext(),
      additionalContext,
    );

    const rawResult = await runEval({
      userPrompt,
      systemMessage,
      userMessage,
      tools,
      helpers,
      apiKey,
      getOutput: getEvalState,
      onAssistantMessageStart: options?.runContext?.onAssistantMessageStart,
      onAssistantToken: options?.runContext?.onAssistantChunk,
      onAssistantMessageEnd: options?.runContext?.onAssistantMessageEnd,
      onToolCall: options?.runContext?.onToolCall,
      options: {
        maxIterations: options?.maxIterations,
        model,
        workspace: workspaceRoot,
        provider: options?.provider,
        transport: options?.transport,
        backend: options?.backend,
        proxyCaseId: options?.runContext?.caseId,
        proxyAttempt: options?.runContext?.attempt,
      },
    });

    return {
      files: rawResult.output,
      success: rawResult.success,
      error: rawResult.error,
      assistantMessageCount: rawResult.iterations,
      toolCallCount: rawResult.toolCallsCount,
      toolsUsed: rawResult.toolsCalled,
      tokenUsage: rawResult.tokenUsage,
    };
  } finally {
    await cleanup();
  }
}

async function buildAdditionalContext(
  appContext: EvalCaseRuntimeAppContextSpec | undefined,
  helpers: AppAIChatHelpers,
): Promise<ContextElement[]> {
  const entries = appContext?.additional ?? [];
  if (entries.length === 0) {
    return [];
  }

  const datatables = entries.some((entry) => entry.type === "datatable")
    ? await helpers.getDatatables()
    : [];

  return entries.map((entry) =>
    buildAdditionalContextElement(entry, helpers, datatables),
  );
}

function buildAdditionalContextElement(
  entry: EvalCaseRuntimeAppAdditionalContext,
  helpers: AppAIChatHelpers,
  datatables: DataTableSchema[],
): ContextElement {
  if (entry.type === "frontend") {
    const content = helpers.getFrontendFile(entry.path);
    if (content === undefined) {
      throw new Error(`App eval @ frontend context not found: ${entry.path}`);
    }
    return createAppFrontendFileContextElement(entry.path, content);
  }

  if (entry.type === "backend") {
    const runnable = helpers.getBackendRunnable(entry.key);
    if (!runnable) {
      throw new Error(`App eval @ backend context not found: ${entry.key}`);
    }
    return createAppBackendRunnableContextElement(entry.key, runnable);
  }

  const datatable = datatables.find(
    (candidate) => candidate.datatable_name === entry.datatableName,
  );
  const columns = datatable?.schemas?.[entry.schema]?.[entry.table];
  if (!columns) {
    throw new Error(
      `App eval @ datatable context not found: ${entry.datatableName}/${entry.schema}.${entry.table}`,
    );
  }

  return createAppDatatableContextElement(
    entry.datatableName,
    entry.schema,
    entry.table,
    columns,
  );
}
