import { mkdtemp, rm } from "fs/promises";
import { tmpdir } from "os";
import { join } from "path";
import type { AIProvider } from "$lib/gen/types.gen";
import {
  globalTools,
  prepareGlobalSystemMessage,
  prepareGlobalUserMessage,
} from "../../../../../frontend/src/lib/components/copilot/chat/global/core";
import {
  clearGlobalDrafts,
  getGlobalDraft,
  listGlobalDrafts,
} from "../../../../../frontend/src/lib/components/copilot/chat/global/userDraftAdapter";
import type { Tool as ProductionTool } from "../../../../../frontend/src/lib/components/copilot/chat/shared";
import { UserDraft } from "../../../../../frontend/src/lib/userDraft.svelte";
import type { ModeRunContext } from "../../../../core/types";
import type { GlobalDraftState } from "../../../../core/validators";
import type { WindmillBackendSettings } from "../../../../core/windmillBackendSettings";
import {
  registerBenchmarkWorkspaceRunnables,
  seedBenchmarkDraft,
  unregisterBenchmarkWorkspaceRunnables,
  type BenchmarkWorkspaceRunnables,
} from "../../mockBackend";
import { runEval } from "../shared";
import type { TokenUsage, ToolCallDetail } from "../shared/types";

const MUTATING_GLOBAL_TOOLS = new Set([
  "deploy_workspace_item",
  "delete_workspace_item",
]);
const DISABLE_ACTIVE_EDITOR_CONTEXT_ENV =
  "WMILL_AI_EVAL_DISABLE_ACTIVE_EDITOR_CONTEXT";
// A/B gate for the search_app read tool: set to "1" to run the baseline arm
// (toolset without search_app) so its token cost can be compared against the arm
// that offers it.
const DISABLE_SEARCH_APP_ENV = "WMILL_AI_EVAL_DISABLE_SEARCH_APP";

const LIVE_EDITOR_ITEM_KINDS = {
  script: "script",
  flow: "flow",
  app: "raw_app",
} as const;

export interface GlobalLiveEditorDraftFixture {
  type: keyof typeof LIVE_EDITOR_ITEM_KINDS;
  storagePath?: string;
  effectivePath?: string;
  value?: unknown;
}

// Identity the global system prompt builds paths from. Production reads
// `userStore` (whoami) to fill `u/{username}/...`; the eval harness never logs
// in, so without this the prompt sees an empty username (`u//...`) and no
// path-selection case is meaningful. Seeded per-case via the initial fixture and
// passed straight to `prepareGlobalSystemMessage` (no global-store mutation).
export interface GlobalUserFixture {
  username: string;
  is_admin?: boolean;
  /** Folders the user can write to (the writable set whoami returns). */
  folders?: string[];
  /** Folders the user can read; read-only folders = folders_read \ folders. */
  folders_read?: string[];
}

export interface GlobalEvalResult {
  success: boolean;
  state: GlobalDraftState;
  error?: string;
  assistantMessageCount: number;
  toolCallCount: number;
  toolsUsed: string[];
  toolCallDetails: ToolCallDetail[];
  tokenUsage: TokenUsage;
  finalContextTokens: number | null;
}

export interface GlobalEvalOptions {
  workspaceFixtures?: BenchmarkWorkspaceRunnables;
  liveEditorDrafts?: GlobalLiveEditorDraftFixture[];
  user?: GlobalUserFixture;
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

  clearGlobalDrafts(workspaceRoot);
  registerBenchmarkWorkspaceRunnables(workspaceRoot, options.workspaceFixtures ?? {});
  seedLiveEditorDrafts(workspaceRoot, options.liveEditorDrafts ?? []);

  try {
    const model = options.model ?? "claude-haiku-4-5-20251001";
    const injectActiveEditorContext =
      process.env[DISABLE_ACTIVE_EDITOR_CONTEXT_ENV] !== "1";
    // Pass the seeded identity straight to the prompt builder rather than mutating
    // the process-global `userStore`, so concurrent cases never race on it.
    const rawResult = await runEval({
      userPrompt,
      systemMessage: prepareGlobalSystemMessage(undefined, { user: options.user }),
      userMessage: prepareGlobalUserMessage(
        userPrompt,
        [],
        injectActiveEditorContext ? { workspace: workspaceRoot } : {},
      ),
      tools: getGlobalEvalTools(),
      helpers: {},
      apiKey,
      getOutput: () => collectGlobalDraftState(workspaceRoot),
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
      finalContextTokens: rawResult.finalContextTokens,
    };
  } finally {
    clearGlobalDrafts(workspaceRoot);
    clearLiveEditorDrafts(workspaceRoot, options.liveEditorDrafts ?? []);
    unregisterBenchmarkWorkspaceRunnables(workspaceRoot);
    if (!options.workspaceRoot) {
      await rm(workspaceRoot, { recursive: true, force: true });
    }
  }
}

// Build the harness output from the DB-backed drafts. `listGlobalDrafts` returns
// metadata-only rows for backend drafts (the model's `write_script` etc. persist
// straight to the backend with no in-tab editor cell), so re-read each such row
// with `getGlobalDraft` to attach the full value the validators assert on. A row
// that already carries a value (the production in-tab cell overlay) is kept as-is.
async function collectGlobalDraftState(
  workspace: string,
): Promise<GlobalDraftState> {
  const items = await listGlobalDrafts(workspace);
  const drafts = await Promise.all(
    items.map(async (item) => {
      if (item.value !== undefined) {
        return item;
      }
      const full = await getGlobalDraft(
        workspace,
        item.type,
        item.path,
        item.triggerKind,
      );
      return full ?? item;
    }),
  );
  return { drafts: drafts as GlobalDraftState["drafts"] };
}

function seedLiveEditorDrafts(
  workspace: string,
  fixtures: GlobalLiveEditorDraftFixture[],
): void {
  for (const fixture of fixtures) {
    const itemKind = LIVE_EDITOR_ITEM_KINDS[fixture.type];
    const storagePath = fixture.storagePath ?? fixture.effectivePath ?? "";
    if (fixture.value !== undefined) {
      // Seed as a backend draft row, not an in-tab cell: a cell would shadow the
      // model's DB-backed edit when the output is read back via listGlobalDrafts.
      seedBenchmarkDraft(workspace, itemKind, storagePath, fixture.value);
    }
    UserDraft.setLiveEditorDraft({
      workspace,
      itemKind,
      storagePath,
      effectivePath: fixture.effectivePath ?? fixture.storagePath,
    });
  }
}

function clearLiveEditorDrafts(
  workspace: string,
  fixtures: GlobalLiveEditorDraftFixture[],
): void {
  for (const fixture of fixtures) {
    const itemKind = LIVE_EDITOR_ITEM_KINDS[fixture.type];
    const storagePath = fixture.storagePath ?? fixture.effectivePath ?? "";
    UserDraft.clearLiveEditorDraft(itemKind, { workspace, storagePath });
  }
}

function getGlobalEvalTools(): ProductionTool<{}>[] {
  const disableSearchApp = process.env[DISABLE_SEARCH_APP_ENV] === "1";
  return (globalTools as ProductionTool<{}>[])
    .filter((tool) => !(disableSearchApp && tool.def.function.name === "search_app"))
    .map((tool) => {
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
