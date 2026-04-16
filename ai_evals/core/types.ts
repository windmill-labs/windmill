export const EVAL_MODES = ["cli", "flow", "script", "app"] as const;

export type EvalMode = (typeof EVAL_MODES)[number];

export interface EvalCaseRuntimeBackendPreview {
  args?: Record<string, unknown>;
  timeoutSeconds?: number;
}

export interface EvalCaseRuntimeSpec {
  backendPreview?: EvalCaseRuntimeBackendPreview;
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
    type: string;
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

export interface EvalCase {
  id: string;
  prompt: string;
  initialPath?: string;
  expectedPath?: string;
  validate?: FlowValidationSpec;
  judgeChecklist?: string[];
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

export interface ModeRunOutput<TActual> {
  success: boolean;
  actual: TActual;
  error?: string;
  assistantMessageCount: number;
  toolCallCount: number;
  toolsUsed: string[];
  skillsInvoked: string[];
  tokenUsage?: BenchmarkTokenUsage | null;
}

export interface ModeRunContext {
  caseId: string;
  caseNumber: number;
  totalCases: number;
  attempt: number;
  runs: number;
  verbose: boolean;
  onAssistantMessageStart?: () => void;
  onAssistantChunk?: (chunk: string) => void;
  onAssistantMessageEnd?: () => void;
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
    context: ModeRunContext
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
}

export interface BenchmarkAttemptResult {
  attempt: number;
  passed: boolean;
  durationMs: number;
  assistantMessageCount: number;
  toolCallCount: number;
  toolsUsed: string[];
  skillsInvoked: string[];
  checks: BenchmarkCheck[];
  judgeScore: number | null;
  judgeSummary: string | null;
  error: string | null;
  tokenUsage?: BenchmarkTokenUsage | null;
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
  totalTokenUsage?: BenchmarkTokenUsage | null;
  averageTokenUsagePerAttempt?: BenchmarkTokenUsage | null;
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
    };
