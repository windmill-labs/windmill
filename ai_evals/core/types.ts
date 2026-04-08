export const EVAL_MODES = ["cli", "flow", "script", "app"] as const;

export type EvalMode = (typeof EVAL_MODES)[number];

export interface EvalCase {
  id: string;
  prompt: string;
  initialPath?: string;
  expectedPath?: string;
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

export interface ModeRunOutput<TActual> {
  success: boolean;
  actual: TActual;
  error?: string;
  assistantMessageCount: number;
  toolCallCount: number;
  toolsUsed: string[];
  skillsInvoked: string[];
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
    prompt: string;
    initial: TInitial | undefined;
    expected: TExpected | undefined;
    actual: TActual;
    run: ModeRunOutput<TActual>;
  }): BenchmarkCheck[];
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
