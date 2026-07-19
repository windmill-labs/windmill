export const EVAL_MODES = ["cli", "flow", "script", "app", "global"] as const;

export type EvalMode = (typeof EVAL_MODES)[number];

export interface EvalCaseRuntimeBackendPreview {
  args?: Record<string, unknown>;
  timeoutSeconds?: number;
}

export type EvalCaseRuntimeAppAdditionalContext =
  | {
      type: "frontend";
      path: string;
    }
  | {
      type: "backend";
      key: string;
    }
  | {
      type: "datatable";
      datatableName: string;
      schema: string;
      table: string;
    };

export interface EvalCaseRuntimeAppContextSpec {
  additional?: EvalCaseRuntimeAppAdditionalContext[];
}

export interface EvalCaseRuntimeSpec {
  maxTurns?: number;
  backendPreview?: EvalCaseRuntimeBackendPreview;
  appContext?: EvalCaseRuntimeAppContextSpec;
  // Global mode: run as a session chat (preview tools + session prompt) vs the standalone chat.
  sessionChat?: boolean;
}

export interface FlowValidationSpec {
  schemaRequiredPaths?: string[];
  schemaAnyOf?: Array<{
    requiredPaths: string[];
  }>;
  exactTopLevelStepIds?: string[];
  topLevelStepIds?: string[];
  topLevelStepOrder?: string[];
  topLevelStepTypeCountsAtLeast?: Array<{
    type: string;
    count: number;
  }>;
  topLevelStepTypes?: Array<{
    id: string;
    type: string | string[];
  }>;
  moduleRules?: Array<{
    id: string;
    hasStopAfterIf?: boolean;
    hasStopAfterAllItersIf?: boolean;
    immediateChildStepIds?: string[];
    exactImmediateChildStepIds?: string[];
    immediateChildStepTypes?: Array<{
      id: string;
      type: string;
    }>;
    requiredInputTransforms?: Array<{
      type?: string;
      expr?: string;
      exprAnyOf?: string[];
      value?: string | number | boolean | null;
    }>;
  }>;
  moduleFieldRules?: Array<{
    id: string;
    path: string;
    equals: string | number | boolean | null;
  }>;
  resolveResultsRefs?: boolean;
  requireSpecialModules?: Array<"preprocessor_module" | "failure_module">;
  requireSuspendSteps?: Array<{
    id: string;
    requiredEvents?: number;
    resumeRequiredStringFieldAnyOf?: string[];
  }>;
}

export interface AppValidationSpec {
  requiredFrontendPaths?: string[];
  requiredFrontendFileContent?: Array<{
    path: string;
    includes: string[];
  }>;
  requiredBackendRunnableKeys?: string[];
  requiredBackendRunnableTypes?: Array<{
    key: string;
    type: string;
  }>;
  requiredBackendRunnableContent?: Array<{
    key: string;
    includes: string[];
  }>;
  backendRunnableCountAtLeast?: number;
  datatableCountAtLeast?: number;
  datatableTableCountAtLeast?: number;
  datatableTableCountExactly?: number;
  requiredDatatables?: Array<{
    schema: string;
    table: string;
    datatableName?: string;
  }>;
  requiredToolsUsed?: string[];
  forbiddenAppContent?: string[];
}

export interface GlobalDraftRequirement {
  type: string;
  path?: string;
  pathIncludes?: string[];
  pathStartsWith?: string;
  triggerKind?: string;
  language?: string;
  summaryIncludes?: string[];
  valueIncludes?: string[];
  valueExcludes?: string[];
}

export interface GlobalValidationSpec {
  draftCountAtLeast?: number;
  draftCountExactly?: number;
  requiredDrafts?: GlobalDraftRequirement[];
  forbiddenDrafts?: Array<{
    type: string;
    path: string;
    triggerKind?: string;
  }>;
}

export interface CliValidationSpec {
  requiredSkills?: string[];
  forbiddenSkills?: string[];
  requiredSkillsBeforeFirstMutation?: string[];
  requiredAssistantMentions?: string[];
  forbiddenAssistantMentions?: string[];
  orderedAssistantMentions?: string[];
  requiredProposedCommands?: string[];
  forbiddenProposedCommands?: string[];
  orderedProposedCommands?: string[];
  forbiddenExecutedCommands?: string[];
  workspaceUnchanged?: boolean;
}

export interface ToolCallDetail {
  name: string;
  arguments: unknown;
}

export interface ToolCallArgumentRule {
  tool: string;
  field: string;
  stringStartsWithAnyOf?: string[];
  stringMustNotStartWithAnyOf?: string[];
  /**
   * Case-insensitive "contains", existential over calls: at least one recorded
   * call to `tool` must have `field` containing one of these substrings. Other
   * calls to the same tool may do anything. Use instead of `stringStartsWithAnyOf`
   * (which is universal over calls) when the meaningful token can appear anywhere
   * in the value and the model may make additional, unrelated calls to the same
   * tool — e.g. SQL where a mutation is mixed with verification SELECTs.
   */
  stringIncludesAnyOf?: string[];
}

export interface ToolValidationSpec {
  requiredToolsUsed?: string[];
  /**
   * Each inner array is an alternatives group: the check passes when at least
   * one tool in the group was used. Use when several tools satisfy the same
   * intent so a model that picks any valid path passes — e.g. inspecting an
   * app's files via either `read_app_file` or `search_app`.
   */
  requiredToolsAnyOf?: string[][];
  forbiddenToolsUsed?: string[];
  toolCallArgs?: ToolCallArgumentRule[];
}

export type EvalValidationSpec =
  | FlowValidationSpec
  | AppValidationSpec
  | GlobalValidationSpec;

export interface EvalCase {
  id: string;
  prompt: string;
  initialPath?: string;
  expectedPath?: string;
  validate?: EvalValidationSpec;
  toolExpect?: ToolValidationSpec;
  cliExpect?: CliValidationSpec;
  judgeChecklist?: string[];
  skipJudge?: boolean;
  runtime?: EvalCaseRuntimeSpec;
}

export interface BenchmarkCheck {
  name: string;
  passed: boolean;
  details?: string;
}

export interface JudgeResult {
  success: boolean;
  score: number;
  summary: string;
  error?: string;
}

export interface BenchmarkArtifactFile {
  path: string;
  content: string;
}

export interface BackendValidationResult {
  checks: BenchmarkCheck[];
  artifactFiles?: BenchmarkArtifactFile[];
}

export interface BenchmarkTokenUsage {
  prompt: number;
  completion: number;
  total: number;
}

export interface CliToolInvocation {
  tool: string;
  input: Record<string, unknown>;
  timestamp: number;
}

export interface CliWmillInvocation {
  argv: string[];
  cwd: string;
  timestamp: string;
}

export interface CliTrace {
  toolsUsed: CliToolInvocation[];
  skillsInvoked: string[];
  assistantMessageCount: number;
  bashCommands: string[];
  proposedCommands: string[];
  executedWmillCommands: string[];
  wmillInvocations: CliWmillInvocation[];
  firstMutationToolIndex: number | null;
}

export interface ModeRunOutput<TActual> {
  success: boolean;
  actual: TActual;
  error?: string;
  assistantMessageCount: number;
  toolCallCount: number;
  toolsUsed: string[];
  toolCallDetails?: ToolCallDetail[];
  skillsInvoked: string[];
  tokenUsage?: BenchmarkTokenUsage | null;
  /**
   * Total input tokens occupying the context window on the LAST model request
   * of the agentic loop (input + cache-creation + cache-read). Complements the
   * cumulative `tokenUsage.prompt`, which sums every iteration's input.
   */
  finalContextTokens?: number | null;
}

export interface ModeRunContext {
  evalCase?: EvalCase;
  caseId: string;
  caseNumber: number;
  totalCases: number;
  attempt: number;
  runs: number;
  verbose: boolean;
  onAssistantMessageStart?: () => void;
  onAssistantChunk?: (chunk: string) => void;
  onAssistantMessageEnd?: () => void;
  onToolCall?: (input: { toolName: string; argumentsText: string }) => void;
}

export interface ModeRunner<TInitial, TExpected, TActual> {
  mode: EvalMode;
  concurrency: number;
  judgeThreshold?: number;
  loadInitial(path?: string): Promise<TInitial | undefined>;
  loadExpected(path?: string): Promise<TExpected | undefined>;
  run(
    prompt: string,
    initial: TInitial | undefined,
    context: ModeRunContext,
  ): Promise<ModeRunOutput<TActual>>;
  validate(input: {
    evalCase: EvalCase;
    prompt: string;
    initial: TInitial | undefined;
    expected: TExpected | undefined;
    actual: TActual;
    run: ModeRunOutput<TActual>;
  }): BenchmarkCheck[];
  backendValidate?(input: {
    evalCase: EvalCase;
    prompt: string;
    initial: TInitial | undefined;
    expected: TExpected | undefined;
    actual: TActual;
    run: ModeRunOutput<TActual>;
    context: ModeRunContext;
  }): Promise<BackendValidationResult | null>;
  buildArtifacts?(actual: TActual): BenchmarkArtifactFile[];
  /**
   * Optional transform applied to `actual` before it is handed to the LLM judge.
   * Use it to strip fields the judge must stay blind to (e.g. which docs-tool
   * arm produced an answer). When omitted, the judge receives `actual` as-is.
   */
  prepareJudgeActual?(actual: TActual): unknown;
}

export interface BenchmarkAttemptResult {
  attempt: number;
  passed: boolean;
  durationMs: number;
  assistantMessageCount: number;
  toolCallCount: number;
  toolsUsed: string[];
  toolCallDetails?: ToolCallDetail[];
  skillsInvoked: string[];
  checks: BenchmarkCheck[];
  judgeScore: number | null;
  judgeSummary: string | null;
  error: string | null;
  tokenUsage?: BenchmarkTokenUsage | null;
  finalContextTokens?: number | null;
  artifactsPath?: string | null;
  artifactFiles?: BenchmarkArtifactFile[];
}

export interface BenchmarkCaseResult {
  id: string;
  prompt: string;
  initialPath?: string;
  expectedPath?: string;
  attempts: BenchmarkAttemptResult[];
}

export interface BenchmarkRunResult {
  version: 1;
  mode: EvalMode;
  createdAt: string;
  gitSha: string | null;
  runs: number;
  runModel: string | null;
  judgeModel: string | null;
  caseCount: number;
  attemptCount: number;
  passedAttempts: number;
  passRate: number;
  averageDurationMs: number;
  averagePassedDurationMs?: number | null;
  totalTokenUsage?: BenchmarkTokenUsage | null;
  totalPassedTokenUsage?: BenchmarkTokenUsage | null;
  averageTokenUsagePerAttempt?: BenchmarkTokenUsage | null;
  averageTokenUsagePerPassedAttempt?: BenchmarkTokenUsage | null;
  averageFinalContextTokensPassed?: number | null;
  maxFinalContextTokensPassed?: number | null;
  artifactsPath?: string | null;
  cases: BenchmarkCaseResult[];
}

export type FrontendBenchmarkProgressEvent =
  | {
      type: "run-start";
      surface: Exclude<EvalMode, "cli">;
      totalCases: number;
      runs: number;
      concurrency: number;
    }
  | {
      type: "attempt-start";
      surface: Exclude<EvalMode, "cli">;
      caseId: string;
      caseNumber: number;
      totalCases: number;
      attempt: number;
      runs: number;
    }
  | {
      type: "attempt-finish";
      surface: Exclude<EvalMode, "cli">;
      caseId: string;
      caseNumber: number;
      totalCases: number;
      attempt: number;
      runs: number;
      passed: boolean;
      durationMs: number;
      judgeScore: number | null;
      error: string | null;
    }
  | {
      type: "assistant-message-start";
      surface: Exclude<EvalMode, "cli">;
      caseId: string;
      caseNumber: number;
      totalCases: number;
      attempt: number;
      runs: number;
    }
  | {
      type: "assistant-chunk";
      surface: Exclude<EvalMode, "cli">;
      caseId: string;
      caseNumber: number;
      totalCases: number;
      attempt: number;
      runs: number;
      chunk: string;
    }
  | {
      type: "assistant-message-end";
      surface: Exclude<EvalMode, "cli">;
      caseId: string;
      caseNumber: number;
      totalCases: number;
      attempt: number;
      runs: number;
    }
  | {
      type: "tool-call";
      surface: Exclude<EvalMode, "cli">;
      caseId: string;
      caseNumber: number;
      totalCases: number;
      attempt: number;
      runs: number;
      toolName: string;
      argumentsText: string;
    };
